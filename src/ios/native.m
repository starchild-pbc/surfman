#include "native.h"

#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#import <IOSurface/IOSurfaceRef.h>
#import <OpenGLES/ES3/gl.h>
#import <OpenGLES/ES3/glext.h>
#import <OpenGLES/EAGL.h>

#include <dlfcn.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>

struct SmIosContext {
  EAGLContext* EaglContext;
  CVOpenGLESTextureCacheRef TextureCache;
  atomic_uint ReferenceCount;
};

struct SmIosSurface {
  CVPixelBufferRef PixelBuffer;
  CVOpenGLESTextureRef Texture;
  GLuint Framebuffer;
  GLuint DepthStencil;
  int32_t Width;
  int32_t Height;
};

struct SmIosSurfaceTexture {
  CVOpenGLESTextureRef Texture;
};

static void sm_ios_surface_clear(SmIosSurface* Surface) {
  if (Surface->Framebuffer != 0) {
    glDeleteFramebuffers(1, &Surface->Framebuffer);
    Surface->Framebuffer = 0;
  }
  if (Surface->DepthStencil != 0) {
    glDeleteRenderbuffers(1, &Surface->DepthStencil);
    Surface->DepthStencil = 0;
  }
  if (Surface->Texture != NULL) {
    CFRelease(Surface->Texture);
    Surface->Texture = NULL;
  }
  if (Surface->PixelBuffer != NULL) {
    CVPixelBufferRelease(Surface->PixelBuffer);
    Surface->PixelBuffer = NULL;
  }
}

static bool sm_ios_surface_initialize(SmIosContext* Context,
                                      SmIosSurface* Surface, int32_t Width,
                                      int32_t Height) {
  NSDictionary* Attributes = @{
    (id)kCVPixelBufferIOSurfacePropertiesKey : @{},
    (id)kCVPixelBufferOpenGLESCompatibilityKey : @YES,
    (id)kCVPixelBufferMetalCompatibilityKey : @YES,
  };
  CVReturn Result = CVPixelBufferCreate(
      kCFAllocatorDefault, Width, Height, kCVPixelFormatType_32BGRA,
      (CFDictionaryRef)Attributes, &Surface->PixelBuffer);
  if (Result != kCVReturnSuccess || Surface->PixelBuffer == NULL) {
    fprintf(stderr, "surfman iOS CVPixelBufferCreate failed. Code %d.\n",
            Result);
    return false;
  }

  Result = CVOpenGLESTextureCacheCreateTextureFromImage(
      kCFAllocatorDefault, Context->TextureCache, Surface->PixelBuffer, NULL,
      GL_TEXTURE_2D, GL_RGBA, Width, Height, GL_BGRA, GL_UNSIGNED_BYTE, 0,
      &Surface->Texture);
  if (Result != kCVReturnSuccess || Surface->Texture == NULL) {
    fprintf(stderr, "surfman iOS texture creation failed. Code %d.\n",
            Result);
    sm_ios_surface_clear(Surface);
    return false;
  }

  const GLuint TextureName = CVOpenGLESTextureGetName(Surface->Texture);
  glBindTexture(GL_TEXTURE_2D, TextureName);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);

  glGenFramebuffers(1, &Surface->Framebuffer);
  glBindFramebuffer(GL_FRAMEBUFFER, Surface->Framebuffer);
  glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                         TextureName, 0);

  glGenRenderbuffers(1, &Surface->DepthStencil);
  glBindRenderbuffer(GL_RENDERBUFFER, Surface->DepthStencil);
  glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH24_STENCIL8, Width, Height);
  glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT,
                            GL_RENDERBUFFER, Surface->DepthStencil);
  glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_STENCIL_ATTACHMENT,
                            GL_RENDERBUFFER, Surface->DepthStencil);
  if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
    fprintf(stderr, "surfman iOS framebuffer creation failed.\n");
    sm_ios_surface_clear(Surface);
    return false;
  }

  Surface->Width = Width;
  Surface->Height = Height;
  return true;
}

SmIosContext* sm_ios_context_create(SmIosContext* SharedContext) {
  @autoreleasepool {
    SmIosContext* Context = calloc(1, sizeof(SmIosContext));
    if (Context == NULL) {
      return NULL;
    }
    atomic_init(&Context->ReferenceCount, 1);
    EAGLSharegroup* Sharegroup =
        SharedContext == NULL ? nil : SharedContext->EaglContext.sharegroup;
    Context->EaglContext = [[EAGLContext alloc]
        initWithAPI:kEAGLRenderingAPIOpenGLES3
         sharegroup:Sharegroup];
    if (Context->EaglContext == nil ||
        ![EAGLContext setCurrentContext:Context->EaglContext]) {
      sm_ios_context_release(Context);
      return NULL;
    }
    const CVReturn Result = CVOpenGLESTextureCacheCreate(
        kCFAllocatorDefault, NULL, Context->EaglContext, NULL,
        &Context->TextureCache);
    if (Result != kCVReturnSuccess || Context->TextureCache == NULL) {
      fprintf(stderr,
              "surfman iOS texture cache creation failed. Code %d.\n",
              Result);
      sm_ios_context_release(Context);
      return NULL;
    }
    return Context;
  }
}

SmIosContext* sm_ios_context_retain(SmIosContext* Context) {
  if (Context != NULL) {
    atomic_fetch_add_explicit(&Context->ReferenceCount, 1,
                              memory_order_relaxed);
  }
  return Context;
}

void sm_ios_context_release(SmIosContext* Context) {
  if (Context == NULL) {
    return;
  }
  if (atomic_load_explicit(&Context->ReferenceCount, memory_order_relaxed) > 1 &&
      atomic_fetch_sub_explicit(&Context->ReferenceCount, 1,
                                memory_order_acq_rel) > 1) {
    return;
  }
  if ([EAGLContext currentContext] == Context->EaglContext) {
    [EAGLContext setCurrentContext:nil];
  }
  if (Context->TextureCache != NULL) {
    CVOpenGLESTextureCacheFlush(Context->TextureCache, 0);
    CFRelease(Context->TextureCache);
  }
  [Context->EaglContext release];
  free(Context);
}

bool sm_ios_context_make_current(SmIosContext* Context) {
  return Context != NULL &&
         [EAGLContext setCurrentContext:Context->EaglContext];
}

bool sm_ios_context_clear_current(void) {
  return [EAGLContext setCurrentContext:nil];
}

const void* sm_ios_gl_proc_address(const char* Name) {
  return dlsym(RTLD_DEFAULT, Name);
}

SmIosSurface* sm_ios_surface_create(SmIosContext* Context, int32_t Width,
                                    int32_t Height) {
  if (Context == NULL || Width <= 0 || Height <= 0 ||
      !sm_ios_context_make_current(Context)) {
    return NULL;
  }
  SmIosSurface* Surface = calloc(1, sizeof(SmIosSurface));
  if (Surface == NULL ||
      !sm_ios_surface_initialize(Context, Surface, Width, Height)) {
    free(Surface);
    return NULL;
  }
  return Surface;
}

void sm_ios_surface_destroy(SmIosContext* Context, SmIosSurface* Surface) {
  if (Surface == NULL) {
    return;
  }
  sm_ios_context_make_current(Context);
  sm_ios_surface_clear(Surface);
  if (Context != NULL && Context->TextureCache != NULL) {
    CVOpenGLESTextureCacheFlush(Context->TextureCache, 0);
  }
  free(Surface);
}

bool sm_ios_surface_resize(SmIosContext* Context, SmIosSurface* Surface,
                           int32_t Width, int32_t Height) {
  if (Context == NULL || Surface == NULL || Width <= 0 || Height <= 0 ||
      !sm_ios_context_make_current(Context)) {
    return false;
  }
  glFinish();
  sm_ios_surface_clear(Surface);
  return sm_ios_surface_initialize(Context, Surface, Width, Height);
}

void sm_ios_surface_bind(SmIosSurface* Surface) {
  if (Surface != NULL) {
    glBindFramebuffer(GL_FRAMEBUFFER, Surface->Framebuffer);
    glViewport(0, 0, Surface->Width, Surface->Height);
  }
}

void sm_ios_surface_finish(void) { glFinish(); }

uint32_t sm_ios_surface_framebuffer(SmIosSurface* Surface) {
  return Surface == NULL ? 0 : Surface->Framebuffer;
}

uint32_t sm_ios_surface_texture(SmIosSurface* Surface) {
  return Surface == NULL ? 0 : CVOpenGLESTextureGetName(Surface->Texture);
}

void* sm_ios_surface_copy_io_surface(SmIosSurface* Surface) {
  if (Surface == NULL) {
    return NULL;
  }
  IOSurfaceRef IoSurface = CVPixelBufferGetIOSurface(Surface->PixelBuffer);
  return IoSurface == NULL ? NULL : (void*)CFRetain(IoSurface);
}

void sm_ios_io_surface_retain(void* Surface) {
  if (Surface != NULL) {
    CFRetain((CFTypeRef)Surface);
  }
}

void sm_ios_io_surface_release(void* Surface) {
  if (Surface != NULL) {
    CFRelease((CFTypeRef)Surface);
  }
}

SmIosSurfaceTexture* sm_ios_surface_texture_create(SmIosContext* Context,
                                                   SmIosSurface* Surface) {
  if (Context == NULL || Surface == NULL ||
      !sm_ios_context_make_current(Context)) {
    return NULL;
  }
  SmIosSurfaceTexture* SurfaceTexture =
      calloc(1, sizeof(SmIosSurfaceTexture));
  if (SurfaceTexture == NULL) {
    return NULL;
  }
  const CVReturn Result = CVOpenGLESTextureCacheCreateTextureFromImage(
      kCFAllocatorDefault, Context->TextureCache, Surface->PixelBuffer, NULL,
      GL_TEXTURE_2D, GL_RGBA, Surface->Width, Surface->Height, GL_BGRA,
      GL_UNSIGNED_BYTE, 0, &SurfaceTexture->Texture);
  if (Result != kCVReturnSuccess || SurfaceTexture->Texture == NULL) {
    free(SurfaceTexture);
    return NULL;
  }
  return SurfaceTexture;
}

void sm_ios_surface_texture_destroy(SmIosSurfaceTexture* Texture) {
  if (Texture != NULL) {
    CFRelease(Texture->Texture);
    free(Texture);
  }
}

uint32_t sm_ios_surface_texture_name(SmIosSurfaceTexture* Texture) {
  return Texture == NULL ? 0 : CVOpenGLESTextureGetName(Texture->Texture);
}

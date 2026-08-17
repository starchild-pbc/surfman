#pragma once

#include <stdbool.h>
#include <stdint.h>

typedef struct SmIosContext SmIosContext;
typedef struct SmIosSurface SmIosSurface;
typedef struct SmIosSurfaceTexture SmIosSurfaceTexture;

SmIosContext* sm_ios_context_create(SmIosContext* SharedContext);
SmIosContext* sm_ios_context_retain(SmIosContext* Context);
void sm_ios_context_release(SmIosContext* Context);
bool sm_ios_context_make_current(SmIosContext* Context);
bool sm_ios_context_clear_current(void);
const void* sm_ios_gl_proc_address(const char* Name);

SmIosSurface* sm_ios_surface_create(SmIosContext* Context, int32_t Width,
                                    int32_t Height);
void sm_ios_surface_destroy(SmIosContext* Context, SmIosSurface* Surface);
bool sm_ios_surface_resize(SmIosContext* Context, SmIosSurface* Surface,
                           int32_t Width, int32_t Height);
void sm_ios_surface_bind(SmIosSurface* Surface);
void sm_ios_surface_finish(void);
uint32_t sm_ios_surface_framebuffer(SmIosSurface* Surface);
uint32_t sm_ios_surface_texture(SmIosSurface* Surface);
void* sm_ios_surface_copy_io_surface(SmIosSurface* Surface);
void sm_ios_io_surface_retain(void* Surface);
void sm_ios_io_surface_release(void* Surface);

SmIosSurfaceTexture* sm_ios_surface_texture_create(SmIosContext* Context,
                                                   SmIosSurface* Surface);
void sm_ios_surface_texture_destroy(SmIosSurfaceTexture* Texture);
uint32_t sm_ios_surface_texture_name(SmIosSurfaceTexture* Texture);

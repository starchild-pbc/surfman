#import <UIKit/UIKit.h>

extern int32_t surfman_ios_device_test_run(void);

static NSString *const PassMarker = @"SURFMAN_IOS_DEVICE_TEST_PASS";
static NSString *const FailMarker = @"SURFMAN_IOS_DEVICE_TEST_FAIL";

@interface SurfmanDeviceTestDelegate : UIResponder <UIApplicationDelegate>
@property(nonatomic, strong) UIWindow *window;
@end

@implementation SurfmanDeviceTestDelegate

- (BOOL)application:(UIApplication *)application
    didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
  UIViewController *viewController = [[UIViewController alloc] init];
  UILabel *label = [[UILabel alloc] initWithFrame:viewController.view.bounds];
  label.autoresizingMask = UIViewAutoresizingFlexibleWidth |
                          UIViewAutoresizingFlexibleHeight;
  label.textAlignment = NSTextAlignmentCenter;
  label.textColor = UIColor.whiteColor;
  label.numberOfLines = 0;
  [viewController.view addSubview:label];

  self.window = [[UIWindow alloc] initWithFrame:UIScreen.mainScreen.bounds];
  self.window.rootViewController = viewController;
  [self.window makeKeyAndVisible];

  int32_t result = surfman_ios_device_test_run();
  if (result == 0) {
    viewController.view.backgroundColor = UIColor.systemGreenColor;
    label.text = PassMarker;
    NSLog(@"%@", PassMarker);
  } else {
    viewController.view.backgroundColor = UIColor.systemRedColor;
    label.text = [NSString stringWithFormat:@"%@\ncode=%d", FailMarker, result];
    NSLog(@"%@ code=%d", FailMarker, result);
  }
  return YES;
}

@end

int main(int argc, char *argv[]) {
  @autoreleasepool {
    return UIApplicationMain(argc, argv, nil,
                             NSStringFromClass(SurfmanDeviceTestDelegate.class));
  }
}

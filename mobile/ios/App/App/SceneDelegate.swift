import UIKit

/// Required as of the iOS 27 SDK: an app that never adopts the UIScene
/// lifecycle now hard-crashes at launch (EXC_BREAKPOINT in
/// UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption) instead of
/// just logging a warning, as it did on earlier SDKs.
///
/// Declaring UIApplicationSceneManifest in Info.plist alone is not enough --
/// the window has to actually be created here, not implicitly via
/// UIMainStoryboardFile/AppDelegate the way this project's original
/// storyboard-only setup did. This loads the same Main.storyboard (whose
/// root view controller is Capacitor's CAPBridgeViewController) so nothing
/// else about the app changes.
class SceneDelegate: UIResponder, UIWindowSceneDelegate {

    var window: UIWindow?

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options connectionOptions: UIScene.ConnectionOptions) {
        guard let windowScene = scene as? UIWindowScene else { return }

        let window = UIWindow(windowScene: windowScene)
        let storyboard = UIStoryboard(name: "Main", bundle: nil)
        window.rootViewController = storyboard.instantiateInitialViewController()
        window.makeKeyAndVisible()
        self.window = window
    }
}

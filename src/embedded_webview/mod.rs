#[cfg(target_os = "windows")]
pub(crate) mod webview2;
#[cfg(target_os = "macos")]
pub(crate) mod wkwebview;
use std::{path::PathBuf, rc::Rc};

use http::Request;
use raw_window_handle::RawWindowHandle;
use url::Url;

use crate::webview::{
  PageLoadEvent, PlatformSpecificWebViewAttributes, ProxyConfig, RequestAsyncResponder, Theme,
  WebContext, RGBA,
};

#[cfg(target_os = "windows")]
use self::webview2::*;
#[cfg(target_os = "macos")]
use self::wkwebview::*;

pub struct EmbeddedWebViewAttributes {
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub x: Option<i32>,
  pub y: Option<i32>,

  /// Whether the WebView should have a custom user-agent.
  pub user_agent: Option<String>,
  /// Whether the WebView window should be visible.
  pub visible: bool,
  /// Whether the WebView should be transparent.
  ///
  /// ## Platform-specific:
  ///
  /// **Windows 7**: Not supported.
  pub transparent: bool,
  /// Specify the webview background color. This will be ignored if `transparent` is set to `true`.
  ///
  /// The color uses the RGBA format.
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS / iOS**: Not implemented.
  /// - **Windows**:
  ///   - On Windows 7, transparency is not supported and the alpha value will be ignored.
  ///   - On Windows higher than 7: translucent colors are not supported so any alpha value other than `0` will be replaced by `255`
  pub background_color: Option<RGBA>,
  /// Whether load the provided URL to [`WebView`].
  pub url: Option<Url>,
  /// Headers used when loading the requested `url`.
  pub headers: Option<http::HeaderMap>,
  /// Whether page zooming by hotkeys is enabled
  ///
  /// ## Platform-specific
  ///
  /// **macOS / Linux / Android / iOS**: Unsupported
  pub zoom_hotkeys_enabled: bool,
  /// Whether load the provided html string to [`WebView`].
  /// This will be ignored if the `url` is provided.
  ///
  /// # Warning
  ///
  /// The Page loaded from html string will have `null` origin.
  ///
  /// ## PLatform-specific:
  ///
  /// - **Windows:** the string can not be larger than 2 MB (2 * 1024 * 1024 bytes) in total size
  pub html: Option<String>,
  /// Initialize javascript code when loading new pages. When webview load a new page, this
  /// initialization code will be executed. It is guaranteed that code is executed before
  /// `window.onload`.
  ///
  /// ## Platform-specific
  ///
  /// - **Android:** The Android WebView does not provide an API for initialization scripts,
  /// so we prepend them to each HTML head. They are only implemented on custom protocol URLs.
  pub initialization_scripts: Vec<String>,
  /// Register custom file loading protocols with pairs of scheme uri string and a handling
  /// closure.
  ///
  /// The closure takes a [Request] and returns a [Response].
  ///
  /// # Warning
  ///
  /// Pages loaded from custom protocol will have different Origin on different platforms. And
  /// servers which enforce CORS will need to add exact same Origin header in `Access-Control-Allow-Origin`
  /// if you wish to send requests with native `fetch` and `XmlHttpRequest` APIs. Here are the
  /// different Origin headers across platforms:
  ///
  /// - macOS, iOS and Linux: `<scheme_name>://<path>` (so it will be `wry://examples` in `custom_protocol` example). On Linux, You need to enable `linux-headers` feature flag.
  /// - Windows and Android: `http://<scheme_name>.<path>` by default (so it will be `http://wry.examples` in `custom_protocol` example). To use `https` instead of `http`, use [`WebViewBuilderExtWindows::with_https_scheme`] and [`WebViewBuilderExtAndroid::with_https_scheme`].
  ///
  /// # Reading assets on mobile
  ///
  /// - Android: Android has `assets` and `resource` path finder to
  /// locate your files in those directories. For more information, see [Loading in-app content](https://developer.android.com/guide/webapps/load-local-content) page.
  /// - iOS: To get the path of your assets, you can call [`CFBundle::resources_path`](https://docs.rs/core-foundation/latest/core_foundation/bundle/struct.CFBundle.html#method.resources_path). So url like `wry://assets/index.html` could get the html file in assets directory.
  ///
  /// [bug]: https://bugs.webkit.org/show_bug.cgi?id=229034
  pub custom_protocols: Vec<(String, Box<dyn Fn(Request<Vec<u8>>, RequestAsyncResponder)>)>,
  /// Set the IPC handler to receive the message from Javascript on webview to host Rust code.
  /// The message sent from webview should call `window.ipc.postMessage("insert_message_here");`.
  pub ipc_handler: Option<Box<dyn Fn(String)>>,

  /// Set a navigation handler to decide if incoming url is allowed to navigate.
  ///
  /// The closure take a `String` parameter as url and return `bool` to determine the url. True is
  /// allow to navigate and false is not.
  pub navigation_handler: Option<Box<dyn Fn(String) -> bool>>,

  /// Set a download started handler to manage incoming downloads.
  ///
  /// The closure takes two parameters - the first is a `String` representing the url being downloaded from and and the
  /// second is a mutable `PathBuf` reference that (possibly) represents where the file will be downloaded to. The latter
  /// parameter can be used to set the download location by assigning a new path to it - the assigned path _must_ be
  /// absolute. The closure returns a `bool` to allow or deny the download.
  pub download_started_handler: Option<Box<dyn FnMut(String, &mut PathBuf) -> bool>>,

  /// Sets a download completion handler to manage downloads that have finished.
  ///
  /// The closure is fired when the download completes, whether it was successful or not.
  /// The closure takes a `String` representing the URL of the original download request, an `Option<PathBuf>`
  /// potentially representing the filesystem path the file was downloaded to, and a `bool` indicating if the download
  /// succeeded. A value of `None` being passed instead of a `PathBuf` does not necessarily indicate that the download
  /// did not succeed, and may instead indicate some other failure - always check the third parameter if you need to
  /// know if the download succeeded.
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS**: The second parameter indicating the path the file was saved to is always empty, due to API
  /// limitations.
  pub download_completed_handler: Option<Rc<dyn Fn(String, Option<PathBuf>, bool) + 'static>>,

  /// Set a new window handler to decide if incoming url is allowed to open in a new window.
  ///
  /// The closure take a `String` parameter as url and return `bool` to determine the url. True is
  /// allow to navigate and false is not.
  pub new_window_req_handler: Option<Box<dyn Fn(String) -> bool>>,

  /// Enables clipboard access for the page rendered on **Linux** and **Windows**.
  ///
  /// macOS doesn't provide such method and is always enabled by default. But you still need to add menu
  /// item accelerators to use shortcuts.
  pub clipboard: bool,

  /// Enable web inspector which is usually called dev tool.
  ///
  /// Note this only enables dev tool to the webview. To open it, you can call
  /// [`WebView::open_devtools`], or right click the page and open it from the context menu.
  ///
  /// ## Platform-specific
  ///
  /// - macOS: This will call private functions on **macOS**. It's still enabled if set in **debug** build on mac,
  /// but requires `devtools` feature flag to actually enable it in **release** build.
  /// - Android: Open `chrome://inspect/#devices` in Chrome to get the devtools window. Wry's `WebView` devtools API isn't supported on Android.
  /// - iOS: Open Safari > Develop > [Your Device Name] > [Your WebView] to get the devtools window.
  pub devtools: bool,
  /// Whether clicking an inactive window also clicks through to the webview. Default is `false`.
  ///
  /// ## Platform-specific
  ///
  /// This configuration only impacts macOS.
  pub accept_first_mouse: bool,

  /// Indicates whether horizontal swipe gestures trigger backward and forward page navigation.
  ///
  /// ## Platform-specific:
  ///
  /// - **Android / iOS:** Unsupported.
  pub back_forward_navigation_gestures: bool,

  /// Run the WebView with incognito mode. Note that WebContext will be ingored if incognito is
  /// enabled.
  ///
  /// ## Platform-specific:
  ///
  /// - **Android:** Unsupported yet.
  pub incognito: bool,

  /// Whether all media can be played without user interaction.
  pub autoplay: bool,

  /// Set a handler closure to process page load events.
  pub on_page_load_handler: Option<Box<dyn Fn(PageLoadEvent, String)>>,

  /// Set a proxy configuration for the webview. Supports HTTP CONNECT and SOCKSv5 proxies
  ///
  /// - **macOS**: Requires macOS 14.0+ and the `mac-proxy` feature flag to be enabled.
  /// - **Android / iOS:** Not supported.
  pub proxy_config: Option<ProxyConfig>,

  /// Whether the webview should be focused when created.
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS / Android / iOS:** Unsupported.
  pub focused: bool,
}

impl Default for EmbeddedWebViewAttributes {
  fn default() -> Self {
    Self {
      width: None,
      height: None,
      x: None,
      y: None,
      user_agent: None,
      visible: true,
      transparent: false,
      background_color: None,
      url: None,
      headers: None,
      html: None,
      initialization_scripts: vec![],
      custom_protocols: vec![],
      ipc_handler: None,
      navigation_handler: None,
      download_started_handler: None,
      download_completed_handler: None,
      new_window_req_handler: None,
      clipboard: false,
      #[cfg(debug_assertions)]
      devtools: true,
      #[cfg(not(debug_assertions))]
      devtools: false,
      zoom_hotkeys_enabled: false,
      accept_first_mouse: false,
      back_forward_navigation_gestures: false,
      incognito: false,
      autoplay: true,
      on_page_load_handler: None,
      proxy_config: None,
      focused: true,
    }
  }
}

pub struct EmbeddedWebViewBuilder<'a> {
  parent: RawWindowHandle,
  pub attrs: EmbeddedWebViewAttributes,
  platform_specific: PlatformSpecificWebViewAttributes,
  web_context: Option<&'a mut WebContext>,
}
impl EmbeddedWebViewBuilder<'_> {
  pub fn new(parent: RawWindowHandle) -> Self {
    Self {
      parent,
      attrs: Default::default(),
      platform_specific: Default::default(),
      web_context: Default::default(),
    }
  }

  pub fn build(self) -> crate::Result<EmbeddedWebview> {
    InnerEmbeddedWebview::new(
      self.parent,
      self.attrs,
      self.platform_specific,
      self.web_context,
    )
    .map(EmbeddedWebview)
  }
}

pub struct EmbeddedWebview(InnerEmbeddedWebview);

impl EmbeddedWebview {
  pub fn new(parent: RawWindowHandle) -> crate::Result<Self> {
    EmbeddedWebViewBuilder::new(parent).build()
  }

  pub fn set_position(&self, x: i32, y: i32) {
    self.0.set_position(x, y)
  }

  /// Get the current url of the webview
  pub fn url(&self) -> Url {
    self.0.url()
  }

  /// Evaluate and run javascript code. Must be called on the same thread who created the
  /// [`WebView`]. Use [`EventLoopProxy`] and a custom event to send scripts from other threads.
  ///
  /// [`EventLoopProxy`]: crate::application::event_loop::EventLoopProxy
  ///
  pub fn evaluate_script(&self, js: &str) -> crate::Result<()> {
    self
      .0
      .eval(js, None::<Box<dyn Fn(String) + Send + 'static>>)
  }

  /// Evaluate and run javascript code with callback function. The evaluation result will be
  /// serialized into a JSON string and passed to the callback function. Must be called on the
  /// same thread who created the [`WebView`]. Use [`EventLoopProxy`] and a custom event to
  /// send scripts from other threads.
  ///
  /// [`EventLoopProxy`]: crate::application::event_loop::EventLoopProxy
  ///
  /// Exception is ignored because of the limitation on windows. You can catch it yourself and return as string as a workaround.
  ///
  /// - ** Android:** Not implemented yet.
  pub fn evaluate_script_with_callback(
    &self,
    js: &str,
    callback: impl Fn(String) + Send + 'static,
  ) -> crate::Result<()> {
    self.0.eval(js, Some(callback))
  }

  /// Launch print modal for the webview content.
  pub fn print(&self) -> crate::Result<()> {
    self.0.print();
    Ok(())
  }

  /// Open the web inspector which is usually called dev tool.
  ///
  /// ## Platform-specific
  ///
  /// - **Android / iOS:** Not supported.
  #[cfg(any(debug_assertions, feature = "devtools"))]
  pub fn open_devtools(&self) {
    self.0.open_devtools();
  }

  /// Close the web inspector which is usually called dev tool.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Not supported.
  #[cfg(any(debug_assertions, feature = "devtools"))]
  pub fn close_devtools(&self) {
    self.0.close_devtools();
  }

  /// Gets the devtool window's current visibility state.
  ///
  /// ## Platform-specific
  ///
  /// - **Windows / Android / iOS:** Not supported.
  #[cfg(any(debug_assertions, feature = "devtools"))]
  pub fn is_devtools_open(&self) -> bool {
    self.0.is_devtools_open()
  }

  /// Set the webview zoom level
  ///
  /// ## Platform-specific:
  ///
  /// - **Android**: Not supported.
  /// - **macOS**: available on macOS 11+ only.
  /// - **iOS**: available on iOS 14+ only.
  pub fn zoom(&self, scale_factor: f64) {
    self.0.zoom(scale_factor);
  }

  /// Specify the webview background color.
  ///
  /// The color uses the RGBA format.
  ///
  /// ## Platfrom-specific:
  ///
  /// - **macOS / iOS**: Not implemented.
  /// - **Windows**:
  ///   - On Windows 7, transparency is not supported and the alpha value will be ignored.
  ///   - On Windows higher than 7: translucent colors are not supported so any alpha value other than `0` will be replaced by `255`
  pub fn set_background_color(&self, background_color: RGBA) -> crate::Result<()> {
    self.0.set_background_color(background_color)
  }

  /// Navigate to the specified url
  pub fn load_url(&self, url: &str) {
    self.0.load_url(url)
  }

  /// Navigate to the specified url using the specified headers
  pub fn load_url_with_headers(&self, url: &str, headers: http::HeaderMap) {
    self.0.load_url_with_headers(url, headers)
  }

  /// Clear all browsing data
  pub fn clear_all_browsing_data(&self) -> crate::Result<()> {
    self.0.clear_all_browsing_data()
  }

  #[cfg(windows)]
  pub fn parent(&self) -> isize {
    self.0.parent.0
  }
}

#[cfg(target_os = "windows")]
impl crate::webview::WebviewExtWindows for EmbeddedWebview {
  fn controller(&self) -> webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller {
    self.0.controller.clone()
  }

  fn set_theme(&self, theme: Theme) {
    self.0.set_theme(theme)
  }
}

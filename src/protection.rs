//! Always-on, local network filtering. No browser extension or Brave installation.
use adblock::{
    lists::{FilterSet, ParseOptions, RuleTypes},
    request::Request,
    Engine,
};

pub struct Protection {
    engine: Engine,
}
impl Protection {
    pub fn bundled() -> Result<Self, String> {
        use std::io::Read;
        let mut rules = String::new();
        flate2::read::GzDecoder::new(include_bytes!("blocklists.txt.gz").as_slice())
            .read_to_string(&mut rules)
            .map_err(|e| e.to_string())?;
        Ok(Self::from_rules(&rules))
    }
    fn from_rules(rules: &str) -> Self {
        let mut filters = FilterSet::new(false);
        filters.add_filter_list(
            rules.to_owned(),
            ParseOptions {
                rule_types: RuleTypes::NetworkOnly,
                ..ParseOptions::default()
            },
        );
        Self {
            engine: Engine::new_with_filter_set(filters),
        }
    }
    pub fn blocks(&self, url: &str, source: &str, kind: &str, method: &str) -> bool {
        let Ok(request) = Request::new(url, source, kind, method) else {
            return false;
        };
        self.engine.check_network_request(&request).should_block()
    }
}

#[cfg(target_os = "windows")]
pub fn attach(view: &wry::WebView, protection: std::rc::Rc<Protection>) -> Result<(), String> {
    use webview2_com::{
        take_pwstr, Microsoft::Web::WebView2::Win32::*, WebResourceRequestedEventHandler,
    };
    use windows::core::{Interface, HSTRING, PWSTR};
    use wry::WebViewExtWindows;
    let core = view.webview();
    let environment = view.environment();
    unsafe {
        if let Ok(extended) = core.cast::<ICoreWebView2_22>() {
            extended
                .AddWebResourceRequestedFilterWithRequestSourceKinds(
                    &HSTRING::from("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )
                .map_err(|e| e.to_string())?;
        } else {
            core.AddWebResourceRequestedFilter(
                &HSTRING::from("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )
            .map_err(|e| e.to_string())?;
        }
        let mut token = 0i64;
        core.add_WebResourceRequested(
            &WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                let (Some(sender), Some(args)) = (sender, args) else {
                    return Ok(());
                };
                let request = args.Request()?;
                let mut raw = PWSTR::null();
                request.Uri(&mut raw)?;
                let url = take_pwstr(raw);
                if !url.starts_with("https://") && !url.starts_with("http://") {
                    return Ok(());
                }
                let mut raw = PWSTR::null();
                sender.Source(&mut raw)?;
                let source = take_pwstr(raw);
                let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                args.ResourceContext(&mut context)?;
                // Never block a top-level page navigation. Requests from within that
                // page are still filtered. Treat frame documents as subdocuments.
                if context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT {
                    let mut destination = PWSTR::null();
                    let header = request
                        .Headers()?
                        .GetHeader(&HSTRING::from("Sec-Fetch-Dest"), &mut destination);
                    let dest = if header.is_ok() {
                        take_pwstr(destination)
                    } else {
                        String::new()
                    };
                    if dest == "document" || url == source || dest.is_empty() {
                        return Ok(());
                    }
                }
                let kind = match context {
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => "script",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => "image",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => "stylesheet",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => "font",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => "media",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST
                    | COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH => "xmlhttprequest",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT => "subdocument",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_PING => "ping",
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET => "websocket",
                    _ => "other",
                };
                let mut raw = PWSTR::null();
                request.Method(&mut raw)?;
                let method = take_pwstr(raw);
                if protection.blocks(&url, &source, kind, &method) {
                    let response = environment.CreateWebResourceResponse(
                        None,
                        403,
                        &HSTRING::from("Blocked by Social Workspace"),
                        &HSTRING::from("Content-Type: text/plain\r\nCache-Control: no-store\r\n"),
                    )?;
                    args.SetResponse(&response)?;
                }
                Ok(())
            })),
            &mut token,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_rules_handle_type_and_site_exceptions() {
        let p=Protection::from_rules("||ads.example^\n||metrics.example^$third-party\n@@||ads.example/needed.js$domain=allowed.example\n||scripts.example^$script");
        assert!(p.blocks(
            "https://ads.example/ad.js",
            "https://news.example",
            "script",
            "GET"
        ));
        assert!(p.blocks(
            "https://metrics.example/pixel",
            "https://news.example",
            "image",
            "GET"
        ));
        assert!(!p.blocks(
            "https://ads.example/needed.js",
            "https://allowed.example",
            "script",
            "GET"
        ));
        assert!(!p.blocks(
            "https://metrics.example/pixel",
            "https://metrics.example",
            "image",
            "GET"
        ));
        assert!(!p.blocks(
            "https://scripts.example/site.css",
            "https://news.example",
            "stylesheet",
            "GET"
        ));
        assert!(p.blocks(
            "https://scripts.example/ad.js",
            "https://news.example",
            "script",
            "GET"
        ));
        assert!(!p.blocks(
            "https://cdn.example/main.js",
            "https://news.example",
            "script",
            "GET"
        ));
    }
    #[test]
    fn bundled_lists_block_known_ad_and_tracker_requests() {
        let p = Protection::bundled().unwrap();
        assert!(p.blocks(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            "https://www.example.org/",
            "script",
            "GET"
        ));
        assert!(p.blocks(
            "https://www.google-analytics.com/analytics.js",
            "https://www.example.org/",
            "script",
            "GET"
        ));
        assert!(!p.blocks(
            "https://www.example.org/assets/main.css",
            "https://www.example.org/",
            "stylesheet",
            "GET"
        ));
        assert!(!p.blocks("not a url", "https://www.example.org/", "other", "GET"));
    }
}

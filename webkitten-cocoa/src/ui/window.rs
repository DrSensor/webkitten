use std::str;

use cocoa::base::{id,nil,NO,YES,BOOL};
use cocoa::foundation::{NSRect, NSPoint, NSSize, NSFastEnumeration,
                        NSAutoreleasePool};
use cocoa::appkit::{NSWindow, NSTitledWindowMask, NSResizableWindowMask,
                    NSMiniaturizableWindowMask, NSClosableWindowMask,
                    NSBackingStoreBuffered};
use cocoa_ext::foundation::{NSArray,NSURLRequest,NSString,NSUInteger};
use cocoa_ext::appkit::{NSLayoutConstraint,NSLayoutAttribute,
                        NSConstraintBasedLayoutInstallingConstraints,
                        NSTextField,NSView,NSControl};
use cocoa_ext::core_graphics::CGRectZero;
use core_graphics::base::CGFloat;
use block::ConcreteBlock;

use webkitten::WEBKITTEN_TITLE;
use webkit::*;
use runtime::{AddressBarDelegate,CommandBarDelegate,log_error_description,nsstring_as_str};
use super::webview;

const BAR_HEIGHT: usize = 24;

pub enum CocoaWindowSubview {
    AddressBar       = 0,
    WebViewContainer = 1,
    CommandBar       = 2,
}

pub fn toggle(window_index: u8, visible: bool) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            match visible {
                true => window.makeKeyAndOrderFront_(nil),
                false => window.orderOut_(nil)
            }
        }
    }
}

pub fn open(uri: Option<&str>) {
    unsafe {
        create_nswindow();
        add_and_focus_webview(window_count() - 1);
    }
}

pub fn focus(window_index: u8) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            window.makeKeyAndOrderFront_(nil);
        }
    }
}

pub fn focused_index() -> u8 {
    unsafe {
        let windows: id = msg_send![super::application::nsapp(), windows];
        for (index, window) in windows.iter().enumerate() {
            let key: BOOL = msg_send![window, isKeyWindow];
            if key == YES {
                return index as u8;
            }
        }
        0
    }
}

pub fn close(window_index: u8) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            window.close();
        }
    }
}

pub fn title(window_index: u8) -> String {
    unsafe {
        window_for_index(window_index)
            .and_then(|win| nsstring_as_str(msg_send![win, title]))
            .and_then(|title| Some(String::from(title)))
            .unwrap_or(String::new())
    }
}

pub fn set_title(window_index: u8, title: &str) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            let title_str = <id as NSString>::from_str(title);
            window.setTitle_(title_str);
        }
    }
}

pub fn window_count() -> u8 {
    unsafe {
        let windows: id = msg_send![super::application::nsapp(), windows];
        windows.count() as u8
    }
}

pub fn open_webview(window_index: u8, uri: &str) {
    unsafe {
        add_and_focus_webview(window_index);
        if let Some(webview) = webview(window_index, focused_webview_index(window_index)) {
            webview::load_uri(webview, uri);
        }
    }
}

pub fn close_webview(window_index: u8, index: u8) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            let container = subview(window, CocoaWindowSubview::WebViewContainer);
            if container.subviews().count() > (index as NSUInteger) {
                container.subviews().object_at_index(index as NSUInteger).remove_from_superview();
            }
        }
    }
}

pub fn focus_webview(window_index: u8, webview_index: u8) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            println!("Focusing webview {} in window {}", webview_index, window_index);
            let expected_index = webview_index as usize;
            for (index, view) in window_webviews(window).iter().enumerate() {
                view.set_hidden(index == expected_index);
            }
        }
    }
}

pub fn webview(window_index: u8, webview_index: u8) -> Option<id> {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            window_webviews(window).get(webview_index as NSUInteger)
        } else {
            None
        }
    }
}

pub fn resize(window_index: u8, width: u32, height: u32) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            let frame = NSRect {
                origin: window.frame().origin,
                size: NSSize { width: width as CGFloat, height: height as CGFloat }
            };
            window.setFrame_display_(frame, YES);
        }
    }
}

pub fn address_field_text(window_index: u8) -> String {
    field_text(window_index, CocoaWindowSubview::AddressBar)
}

pub fn set_address_field_text(window_index: u8, text: &str) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            let bar = subview(window, CocoaWindowSubview::AddressBar);
            bar.set_string_value(text);
        }
    }
}

pub fn command_field_text(window_index: u8) -> String {
    field_text(window_index, CocoaWindowSubview::CommandBar)
}

fn field_text(window_index: u8, view: CocoaWindowSubview) -> String {
    unsafe {
        window_for_index(window_index)
            .and_then(|window| {
                let field = subview(window, view);
                let text: id = field.string_value();
                nsstring_as_str(text) })
            .and_then(|text| Some(String::from(text)))
            .unwrap_or(String::new())
    }
}

pub fn set_command_field_text(window_index: u8, text: &str) {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            let bar = subview(window, CocoaWindowSubview::CommandBar);
            bar.set_string_value(text);
        }
    }
}

pub fn focused_webview_index(window_index: u8) -> u8 {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            for (index, view) in window_webviews(window).iter().enumerate() {
                if view.hidden() == NO {
                    return index as u8;
                }
            }
        }
    }
    0
}

pub fn webview_count(window_index: u8) -> u8 {
    unsafe {
        if let Some(window) = window_for_index(window_index) {
            window_webviews(window).count() as u8
        } else {
            0
        }
    }
}

unsafe fn window_for_index(index: u8) -> Option<id> {
    let windows: id = msg_send![super::application::nsapp(), windows];
    windows.get(index as NSUInteger)
}

unsafe fn subview(window: id, index: CocoaWindowSubview) -> id {
    let subviews = window.contentView().subviews();
    msg_send![subviews, objectAtIndex:index]
}

unsafe fn add_and_focus_webview(window_index: u8) {
    let store = _WKUserContentExtensionStore::default_store(nil);
    let block = ConcreteBlock::new(move |filter: id, err: id| {
        if let Some(window) = window_for_index(window_index) {
            let container = subview(window, CocoaWindowSubview::WebViewContainer);
            for view in container.subviews().iter() {
                view.set_hidden(true);
            }
            let config = WKWebViewConfiguration().autorelease();
            if err == nil {
                config.user_content_controller().add_user_content_filter(filter);
            } else {
                log_error_description(err);
            }
            let webview = WKWebView(CGRectZero(), config).autorelease();
            webview.disable_translates_autoresizing_mask_into_constraints();
            container.add_subview(webview);
            container.add_constraint(<id as NSLayoutConstraint>::bind(webview, NSLayoutAttribute::Top, container, NSLayoutAttribute::Top));
            container.add_constraint(<id as NSLayoutConstraint>::bind(webview, NSLayoutAttribute::Bottom, container, NSLayoutAttribute::Bottom));
            container.add_constraint(<id as NSLayoutConstraint>::bind(webview, NSLayoutAttribute::Left, container, NSLayoutAttribute::Left));
            container.add_constraint(<id as NSLayoutConstraint>::bind(webview, NSLayoutAttribute::Right, container, NSLayoutAttribute::Right));
        }
    });
    store.lookup_content_extension("filter", &block.copy());
}

unsafe fn window_webviews(window: id) -> id {
    subview(window, CocoaWindowSubview::WebViewContainer).subviews()
}

unsafe fn create_nswindow() -> id {
    let mask = (NSTitledWindowMask as NSUInteger |
                NSMiniaturizableWindowMask as NSUInteger |
                NSResizableWindowMask as NSUInteger |
                NSClosableWindowMask as NSUInteger) as NSUInteger;
    let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
        NSRect::new(NSPoint::new(0., 0.), NSSize::new(700., 700.)),
        mask,
        NSBackingStoreBuffered,
        NO
    ).autorelease();
    window.cascadeTopLeftFromPoint_(NSPoint::new(20., 20.));
    window.center();
    let title = <id as NSString>::from_str(WEBKITTEN_TITLE);
    window.setTitle_(title);
    layout_window_subviews(window);
    window
}

unsafe fn layout_window_subviews(window: id) -> (id, id) {
    let container = <id as NSView>::new();
    let address_bar = <id as NSTextField>::new();
    let command_bar = <id as NSTextField>::new();
    window.contentView().add_subview(address_bar);
    window.contentView().add_subview(container);
    window.contentView().add_subview(command_bar);
    address_bar.disable_translates_autoresizing_mask_into_constraints();
    address_bar.set_height(BAR_HEIGHT as CGFloat);
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(address_bar, NSLayoutAttribute::Top, window.contentView(), NSLayoutAttribute::Top));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(address_bar, NSLayoutAttribute::Left, window.contentView(), NSLayoutAttribute::Left));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(address_bar, NSLayoutAttribute::Right, window.contentView(), NSLayoutAttribute::Right));
    command_bar.disable_translates_autoresizing_mask_into_constraints();
    command_bar.set_height(BAR_HEIGHT as CGFloat);
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(command_bar, NSLayoutAttribute::Bottom, window.contentView(), NSLayoutAttribute::Bottom));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(command_bar, NSLayoutAttribute::Left, window.contentView(), NSLayoutAttribute::Left));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(command_bar, NSLayoutAttribute::Right, window.contentView(), NSLayoutAttribute::Right));
    container.disable_translates_autoresizing_mask_into_constraints();
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(container, NSLayoutAttribute::Top, address_bar, NSLayoutAttribute::Bottom));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(container, NSLayoutAttribute::Bottom, command_bar, NSLayoutAttribute::Top));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(container, NSLayoutAttribute::Left, window.contentView(), NSLayoutAttribute::Left));
    window.contentView().add_constraint(<id as NSLayoutConstraint>::bind(container, NSLayoutAttribute::Right, window.contentView(), NSLayoutAttribute::Right));
    window.makeKeyAndOrderFront_(nil);
    let address_bar_delegate: id = AddressBarDelegate::new();
    let command_bar_delegate: id = CommandBarDelegate::new();
    address_bar.set_delegate(address_bar_delegate);
    command_bar.set_delegate(command_bar_delegate);
    (address_bar_delegate, command_bar_delegate)
}

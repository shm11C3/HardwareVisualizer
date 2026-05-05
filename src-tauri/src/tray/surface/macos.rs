use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{
  AnyThread, ClassType, DeclaredClass, MainThreadOnly, define_class, msg_send, sel,
};
use objc2_app_kit::{
  NSAttributedStringAttachmentConveniences, NSColor, NSFont, NSFontAttributeName,
  NSForegroundColorAttributeName, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
  NSTextAttachment, NSVariableStatusItemLength, NSView,
};
use objc2_foundation::{
  MainThreadMarker, NSAttributedString, NSData, NSMutableAttributedString, NSPoint,
  NSRange, NSRect, NSSize, NSString,
};
use tauri::AppHandle;

use crate::log_warn;
use crate::tray::widget::{MetricState, TrayFrame, TrayMetricIcon};

use super::TraySurface;

const APP_LOGO_SVG: &[u8] = include_bytes!("../../../../assets/hv-icon-outline.svg");
const APP_LOGO_SIZE: f64 = 18.0;
const METRIC_ICON_SIZE: f64 = 13.0;

pub struct MacosTraySurface {
  app_handle: AppHandle,
  state: Arc<MainThreadState>,
}

impl MacosTraySurface {
  pub fn new(app_handle: AppHandle) -> tauri::Result<Self> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
      tauri::Error::Io(std::io::Error::other(
        "native macOS tray surface must be created on the main thread",
      ))
    })?;
    let native = NativeMacosStatusItem::new(app_handle.clone(), mtm)?;

    Ok(Self {
      app_handle,
      state: Arc::new(MainThreadState {
        item: Mutex::new(Some(native)),
      }),
    })
  }

  fn run_on_main_thread<F>(&self, label: &'static str, task: F)
  where
    F: FnOnce(&mut NativeMacosStatusItem) + Send + 'static,
  {
    let state = Arc::clone(&self.state);
    if let Err(e) = self.app_handle.run_on_main_thread(move || {
      if let Some(item) = state.item.lock().unwrap().as_mut() {
        task(item);
      }
    }) {
      log_warn!(
        &format!("failed to schedule macOS tray {label}: {e}"),
        "tray::surface::macos",
        None::<&str>
      );
    }
  }
}

impl TraySurface for MacosTraySurface {
  fn apply_frame(&self, frame: &TrayFrame) {
    let frame = frame.clone();
    self.run_on_main_thread("frame update", move |item| item.apply_frame(&frame));
  }

  fn close(&self) {
    let state = Arc::clone(&self.state);
    if let Err(e) = self.app_handle.run_on_main_thread(move || {
      state.item.lock().unwrap().take();
    }) {
      log_warn!(
        &format!("failed to schedule macOS tray removal: {e}"),
        "tray::surface::macos",
        None::<&str>
      );
    }
  }
}

impl Drop for MacosTraySurface {
  fn drop(&mut self) {
    self.close();
  }
}

struct MainThreadState {
  item: Mutex<Option<NativeMacosStatusItem>>,
}

impl Drop for MainThreadState {
  fn drop(&mut self) {
    let Ok(item) = self.item.get_mut() else {
      return;
    };

    if item.is_some() && MainThreadMarker::new().is_none() {
      // SAFETY: Dropping AppKit objects off the main thread can call into
      // Objective-C dealloc paths on the wrong thread. If Tauri can no
      // longer schedule the normal close path, leaking the status item is
      // safer; the process is already shutting down in that case.
      std::mem::forget(item.take());
    }
  }
}

// SAFETY: `NativeMacosStatusItem` contains AppKit objects, which are only
// touched inside closures scheduled via `AppHandle::run_on_main_thread`.
// The mutex only guards ownership transfer into those closures; it is not
// permission to use the AppKit objects off the main thread.
unsafe impl Send for MainThreadState {}

// SAFETY: Shared access follows the same rule as `Send`: every read/write
// of the contained AppKit object is routed back to the main thread first.
unsafe impl Sync for MainThreadState {}

struct NativeMacosStatusItem {
  status_item: Retained<NSStatusItem>,
  target: Retained<MacosTrayTarget>,
  _menu: Retained<NSMenu>,
}

impl NativeMacosStatusItem {
  fn new(app_handle: AppHandle, mtm: MainThreadMarker) -> tauri::Result<Self> {
    let status_item =
      NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    let menu = build_menu(&app_handle, mtm);

    status_item.setMenu(Some(&menu));

    let button = status_item.button(mtm).ok_or_else(|| {
      tauri::Error::Io(std::io::Error::other(
        "failed to resolve macOS status item button",
      ))
    })?;
    button.setAttributedTitle(&empty_attributed_title());
    button.setToolTip(Some(&NSString::from_str("HardwareVisualizer")));

    let target = mtm.alloc().set_ivars(MacosTrayTargetIvars {
      app_handle,
      menu: menu.clone(),
      status_item: status_item.clone(),
    });
    let frame = button.frame();
    let target: Retained<MacosTrayTarget> =
      unsafe { msg_send![super(target), initWithFrame: frame] };
    target.setWantsLayer(true);
    button.addSubview(&target);

    Ok(Self {
      status_item,
      target,
      _menu: menu,
    })
  }

  fn apply_frame(&mut self, frame: &TrayFrame) {
    let Some(button) = self
      .status_item
      .button(MainThreadMarker::from(&*self.target))
    else {
      return;
    };

    let title = attributed_title(frame);
    button.setAttributedTitle(&title);
    button.setToolTip(Some(&NSString::from_str(&frame.tooltip)));
    self.target.update_dimensions();
  }
}

impl Drop for NativeMacosStatusItem {
  fn drop(&mut self) {
    self.target.removeFromSuperview();
    NSStatusBar::systemStatusBar().removeStatusItem(&self.status_item);
  }
}

fn build_menu(app_handle: &AppHandle, mtm: MainThreadMarker) -> Retained<NSMenu> {
  let menu_title = NSString::from_str("HardwareVisualizer");
  let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &menu_title);
  let open_item = MacosTrayMenuItem::create(
    mtm,
    app_handle.clone(),
    "Open",
    MacosTrayMenuCommand::Open,
  );
  let separator = NSMenuItem::separatorItem(mtm);
  let quit_item = MacosTrayMenuItem::create(
    mtm,
    app_handle.clone(),
    "Quit",
    MacosTrayMenuCommand::Quit,
  );

  menu.addItem(&open_item);
  menu.addItem(&separator);
  menu.addItem(&quit_item);

  menu
}

fn attributed_title(frame: &TrayFrame) -> Retained<objc2_foundation::NSAttributedString> {
  let (attributed, value_ranges) = build_attributed_title(frame);
  let full_range = NSRange::new(0, attributed.length());
  let label_color: Retained<AnyObject> = NSColor::labelColor().into();
  let font: Retained<AnyObject> =
    NSFont::systemFontOfSize(NSFont::systemFontSize()).into();

  // SAFETY: These AppKit attribute keys expect NSColor / NSFont objects,
  // and `full_range` covers the attributed string that was just built.
  unsafe {
    attributed.addAttribute_value_range(
      NSForegroundColorAttributeName,
      &label_color,
      full_range,
    );
    attributed.addAttribute_value_range(NSFontAttributeName, &font, full_range);
  }

  for (range, state) in value_ranges {
    let color: Retained<AnyObject> = match state {
      MetricState::Normal => NSColor::labelColor().into(),
      MetricState::Warning => NSColor::systemYellowColor().into(),
      MetricState::Critical => NSColor::systemRedColor().into(),
    };

    // SAFETY: The attribute value is an NSColor and the range was
    // produced while building the same NSString content.
    unsafe {
      attributed.addAttribute_value_range(NSForegroundColorAttributeName, &color, range);
    }
  }

  Retained::into_super(attributed)
}

fn build_attributed_title(
  frame: &TrayFrame,
) -> (
  Retained<NSMutableAttributedString>,
  Vec<(NSRange, MetricState)>,
) {
  let attributed = NSMutableAttributedString::from_nsstring(&NSString::from_str(""));
  append_app_logo(&attributed);

  if frame.items.is_empty() {
    return (attributed, vec![]);
  }

  let mut value_ranges = Vec::with_capacity(frame.items.len());

  for item in &frame.items {
    append_text(&attributed, "  ");
    append_metric_icon(&attributed, item.icon);
    append_text(&attributed, " ");

    let value_start = attributed.length();
    append_text(&attributed, &item.value);
    let value_len = attributed.length() - value_start;
    value_ranges.push((NSRange::new(value_start, value_len), item.state));
  }

  (attributed, value_ranges)
}

fn append_metric_icon(attributed: &NSMutableAttributedString, icon: TrayMetricIcon) {
  let Some(image) = system_symbol_image(icon) else {
    append_text(attributed, icon.fallback_label);
    return;
  };

  image.setTemplate(true);

  let attachment = NSTextAttachment::new();
  attachment.setImage(Some(&image));
  attachment.setBounds(NSRect::new(
    NSPoint::new(0.0, -2.0),
    NSSize::new(METRIC_ICON_SIZE, METRIC_ICON_SIZE),
  ));

  let symbol = NSAttributedString::attributedStringWithAttachment(&attachment);
  attributed.appendAttributedString(&symbol);
}

fn append_app_logo(attributed: &NSMutableAttributedString) {
  let Some(image) = app_logo_image() else {
    append_text(attributed, "HV");
    return;
  };

  let attachment = NSTextAttachment::new();
  attachment.setImage(Some(&image));
  attachment.setBounds(NSRect::new(
    NSPoint::new(0.0, -4.0),
    NSSize::new(APP_LOGO_SIZE, APP_LOGO_SIZE),
  ));

  let logo = NSAttributedString::attributedStringWithAttachment(&attachment);
  attributed.appendAttributedString(&logo);
}

fn app_logo_image() -> Option<Retained<NSImage>> {
  let data = NSData::from_vec(APP_LOGO_SVG.to_vec());
  let image = NSImage::initWithData(NSImage::alloc(), &data)?;
  image.setTemplate(true);
  image.setSize(NSSize::new(APP_LOGO_SIZE, APP_LOGO_SIZE));
  Some(image)
}

fn empty_attributed_title() -> Retained<objc2_foundation::NSAttributedString> {
  let attributed = NSMutableAttributedString::from_nsstring(&NSString::from_str(""));
  append_app_logo(&attributed);
  Retained::into_super(attributed)
}

fn system_symbol_image(icon: TrayMetricIcon) -> Option<Retained<NSImage>> {
  if !NSImage::class()
    .metaclass()
    .responds_to(sel!(imageWithSystemSymbolName:accessibilityDescription:))
  {
    return None;
  }

  NSImage::imageWithSystemSymbolName_accessibilityDescription(
    &NSString::from_str(icon.macos_symbol_name),
    Some(&NSString::from_str(icon.accessibility_label)),
  )
}

fn append_text(attributed: &NSMutableAttributedString, text: &str) {
  let text = NSAttributedString::from_nsstring(&NSString::from_str(text));
  attributed.appendAttributedString(&text);
}

#[cfg(test)]
mod tests {
  use crate::tray::widget::{TrayFrameItem, TrayMetric, TrayMetricIcon};

  use super::*;

  #[test]
  fn title_parts_keep_status_item_visible_without_metrics() {
    let frame = TrayFrame {
      title: None,
      items: vec![],
      tooltip: "HardwareVisualizer".to_string(),
    };
    let (attributed, ranges) = build_attributed_title(&frame);

    assert_eq!(attributed.length(), 1);
    assert_ne!(&attributed.string().to_string(), "HV");
    assert_eq!(ranges, vec![]);
  }

  #[test]
  fn title_parts_keep_app_logo_when_metrics_are_visible() {
    let frame = TrayFrame {
      title: Some("CPU 42%".to_string()),
      items: vec![TrayFrameItem {
        metric: TrayMetric::Cpu,
        icon: TrayMetricIcon {
          fallback_label: "CPU",
          macos_symbol_name: "missing.hardwarevisualizer.cpu",
          accessibility_label: "CPU usage",
        },
        value: "42%".to_string(),
        state: MetricState::Normal,
      }],
      tooltip: "CPU usage: 42% (normal)".to_string(),
    };
    let (attributed, ranges) = build_attributed_title(&frame);
    let title = attributed.string().to_string();

    assert_eq!(title, "\u{fffc}  CPU 42%");
    assert_eq!(title.matches('\u{fffc}').count(), 1);
    assert_eq!(ranges, vec![(NSRange::new(7, 3), MetricState::Normal)]);
  }

  #[test]
  fn app_logo_image_is_template_for_label_color_tinting() {
    let image = app_logo_image().expect("app logo image should load from bundled SVG");

    assert!(image.isTemplate());
  }

  #[test]
  fn title_parts_color_only_metric_values() {
    let cpu_icon = TrayMetricIcon {
      fallback_label: "CPU",
      macos_symbol_name: "missing.hardwarevisualizer.cpu",
      accessibility_label: "CPU usage",
    };
    let gpu_icon = TrayMetricIcon {
      fallback_label: "GPU",
      macos_symbol_name: "missing.hardwarevisualizer.gpu",
      accessibility_label: "GPU usage",
    };
    let frame = TrayFrame {
      title: Some("CPU 72%!  GPU 91%!!".to_string()),
      items: vec![
        TrayFrameItem {
          metric: TrayMetric::Cpu,
          icon: cpu_icon,
          value: "72%".to_string(),
          state: MetricState::Warning,
        },
        TrayFrameItem {
          metric: TrayMetric::Gpu,
          icon: gpu_icon,
          value: "91%".to_string(),
          state: MetricState::Critical,
        },
      ],
      tooltip: "CPU usage: 72% (warning)\nGPU usage: 91% (critical)".to_string(),
    };
    let (attributed, ranges) = build_attributed_title(&frame);

    assert_eq!(
      &attributed.string().to_string(),
      "\u{fffc}  CPU 72%  GPU 91%"
    );
    assert_eq!(
      ranges,
      vec![
        (NSRange::new(7, 3), MetricState::Warning),
        (NSRange::new(16, 3), MetricState::Critical),
      ]
    );
  }
}

#[derive(Clone, Copy)]
enum MacosTrayMenuCommand {
  Open,
  Quit,
}

struct MacosTrayMenuItemIvars {
  app_handle: AppHandle,
  command: MacosTrayMenuCommand,
}

define_class!(
  #[unsafe(super(NSMenuItem))]
  #[name = "HardwareVisualizerTrayMenuItem"]
  #[thread_kind = MainThreadOnly]
  #[ivars = MacosTrayMenuItemIvars]
  struct MacosTrayMenuItem;

  impl MacosTrayMenuItem {
    #[unsafe(method(perform:))]
    fn perform(&self, _sender: Option<&AnyObject>) {
      match self.ivars().command {
        MacosTrayMenuCommand::Open => {
          crate::lifecycle::restore_main_window(&self.ivars().app_handle);
        }
        MacosTrayMenuCommand::Quit => {
          crate::adapters::tray::spawn_quit(self.ivars().app_handle.clone());
        }
      }
    }
  }
);

impl MacosTrayMenuItem {
  fn create(
    mtm: MainThreadMarker,
    app_handle: AppHandle,
    title: &str,
    command: MacosTrayMenuCommand,
  ) -> Retained<NSMenuItem> {
    let item = mtm.alloc().set_ivars(MacosTrayMenuItemIvars {
      app_handle,
      command,
    });
    let title = NSString::from_str(title);
    let key_equivalent = NSString::from_str("");
    let item: Retained<MacosTrayMenuItem> = unsafe {
      msg_send![super(item), initWithTitle: &*title, action: Some(sel!(perform:)), keyEquivalent: &*key_equivalent]
    };

    // SAFETY: The selector is implemented by `MacosTrayMenuItem`, and
    // NSMenuItem keeps a weak target reference while we retain the item
    // through its parent menu for the status item's lifetime.
    unsafe {
      item.setTarget(Some(&item));
      item.setEnabled(true);
    }

    Retained::into_super(item)
  }
}

struct MacosTrayTargetIvars {
  app_handle: AppHandle,
  menu: Retained<NSMenu>,
  status_item: Retained<NSStatusItem>,
}

define_class!(
  #[unsafe(super(NSView))]
  #[name = "HardwareVisualizerTrayTarget"]
  #[thread_kind = MainThreadOnly]
  #[ivars = MacosTrayTargetIvars]
  struct MacosTrayTarget;

  impl MacosTrayTarget {
    #[unsafe(method(mouseDown:))]
    fn mouse_down(&self, _event: &objc2_app_kit::NSEvent) {
      self.highlight(true);
    }

    #[unsafe(method(mouseUp:))]
    fn mouse_up(&self, _event: &objc2_app_kit::NSEvent) {
      self.highlight(false);
      crate::lifecycle::restore_main_window(&self.ivars().app_handle);
    }

    #[unsafe(method(rightMouseDown:))]
    fn right_mouse_down(&self, _event: &objc2_app_kit::NSEvent) {
      self.open_menu();
    }

    #[unsafe(method(rightMouseUp:))]
    fn right_mouse_up(&self, _event: &objc2_app_kit::NSEvent) {
      self.highlight(false);
    }
  }
);

impl MacosTrayTarget {
  fn update_dimensions(&self) {
    let mtm = MainThreadMarker::from(self);
    if let Some(button) = self.ivars().status_item.button(mtm) {
      self.setFrame(button.frame());
    }
  }

  fn highlight(&self, highlighted: bool) {
    let mtm = MainThreadMarker::from(self);
    if let Some(button) = self.ivars().status_item.button(mtm) {
      button.highlight(highlighted);
    }
  }

  fn open_menu(&self) {
    if self.ivars().menu.numberOfItems() == 0 {
      return;
    }

    let mtm = MainThreadMarker::from(self);
    if let Some(button) = self.ivars().status_item.button(mtm) {
      // SAFETY: The button and menu belong to this live NSStatusItem and
      // this method always runs on the AppKit main thread.
      unsafe {
        button.performClick(None);
      }
    }
  }
}

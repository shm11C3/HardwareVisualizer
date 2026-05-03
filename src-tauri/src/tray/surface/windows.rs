use std::{
  collections::{HashMap, VecDeque},
  sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::{
  AppHandle, Emitter, Listener, Manager, Monitor, PhysicalPosition, PhysicalSize, Rect,
  WebviewUrl, WebviewWindow, WebviewWindowBuilder,
  image::Image,
  menu::{Menu, MenuEvent, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

use crate::log_warn;
use crate::tray::TRAY_WIDGET_FLYOUT_LABEL;
use crate::tray::widget::{MetricState, TrayFrame, TrayFrameItem, TrayMetric};

use super::TraySurface;

const MENU_OPEN_ID: &str = "tray::open";
const MENU_QUIT_ID: &str = "tray::quit";

const ICON_SIZE: u32 = 64;
const ICON_CACHE_LIMIT: usize = 3;
const FLYOUT_WIDTH: f64 = 300.0;
const FLYOUT_HEIGHT: f64 = 148.0;

const ICON_COLOR: Rgba = Rgba([245, 247, 250, 255]);
const WARNING_COLOR: Rgba = Rgba([255, 185, 0, 255]);
const CRITICAL_COLOR: Rgba = Rgba([255, 69, 58, 255]);
const FLYOUT_OFFSET: f64 = 12.0;
const EVENT_TRAY_WIDGET_FRAME: &str = "tray-widget-frame";
const EVENT_TRAY_WIDGET_OPEN_MAIN_REQUESTED: &str = "tray-widget-open-main-requested";
const EVENT_TRAY_WIDGET_HIDE_REQUESTED: &str = "tray-widget-hide-requested";

pub struct WindowsTraySurface {
  app_handle: AppHandle,
  tray: TrayIcon,
  state: Arc<WindowsTrayState>,
}

#[derive(Default)]
struct WindowsTrayState {
  cache: Mutex<WindowsIconCache>,
  latest_frame: Mutex<Option<WindowsFlyoutFrame>>,
}

impl WindowsTraySurface {
  pub fn new(app: &tauri::App) -> tauri::Result<Self> {
    let app_handle = app.handle().clone();
    ensure_flyout_window(&app_handle)?;
    register_flyout_event_handlers(&app_handle);

    let open_item = MenuItem::with_id(app, MENU_OPEN_ID, "Open", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let icon = render_hv_outline_image(MetricState::Normal);

    let state = Arc::new(WindowsTrayState::default());
    let event_app_handle = app_handle.clone();
    let event_state = Arc::clone(&state);

    let tray = TrayIconBuilder::new()
      .icon(icon.clone())
      .menu(&menu)
      .show_menu_on_left_click(false)
      .on_menu_event(handle_menu_event)
      .on_tray_icon_event(move |_tray, event| {
        handle_tray_event(&event_app_handle, &event_state, event);
      })
      .build(app)?;

    Ok(Self {
      app_handle,
      tray,
      state,
    })
  }
}

impl TraySurface for WindowsTraySurface {
  fn apply_frame(&self, frame: &TrayFrame) {
    let flyout_frame = WindowsFlyoutFrame::from_frame(frame);
    self
      .state
      .latest_frame
      .lock()
      .unwrap()
      .replace(flyout_frame.clone());
    emit_flyout_frame(&self.app_handle, &flyout_frame);

    if let Err(e) = self.tray.set_tooltip(Some(frame.tooltip.as_str())) {
      log_warn!(
        &format!("failed to update Windows tray widget tooltip: {e}"),
        "tray::surface::windows::apply_frame",
        None::<&str>
      );
    }

    let icon = self.state.cache.lock().unwrap().image_for_frame(frame);

    if let Err(e) = self.tray.set_icon(Some(icon)) {
      log_warn!(
        &format!("failed to update Windows tray widget icon: {e}"),
        "tray::surface::windows::apply_frame",
        None::<&str>
      );
    }
  }

  fn close(&self) {}
}

fn ensure_flyout_window(app_handle: &AppHandle) -> tauri::Result<WebviewWindow> {
  if let Some(window) = app_handle.get_webview_window(TRAY_WIDGET_FLYOUT_LABEL) {
    return Ok(window);
  }

  WebviewWindowBuilder::new(
    app_handle,
    TRAY_WIDGET_FLYOUT_LABEL,
    WebviewUrl::App("index.html".into()),
  )
  .title("HardwareVisualizer")
  .inner_size(FLYOUT_WIDTH, FLYOUT_HEIGHT)
  .resizable(false)
  .decorations(false)
  .skip_taskbar(true)
  .always_on_top(true)
  .focused(false)
  .visible(false)
  .shadow(true)
  .build()
}

fn register_flyout_event_handlers(app_handle: &AppHandle) {
  let open_app = app_handle.clone();
  app_handle.listen_any(EVENT_TRAY_WIDGET_OPEN_MAIN_REQUESTED, move |_| {
    crate::adapters::tray::restore_main_window(&open_app);
    hide_flyout_window(&open_app);
  });

  let hide_app = app_handle.clone();
  app_handle.listen_any(EVENT_TRAY_WIDGET_HIDE_REQUESTED, move |_| {
    hide_flyout_window(&hide_app);
  });
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
  match event.id.as_ref() {
    MENU_OPEN_ID => crate::adapters::tray::restore_main_window(app),
    MENU_QUIT_ID => crate::adapters::tray::spawn_quit(app.clone()),
    other => {
      log_warn!(
        &format!("unhandled tray menu event: {other}"),
        "tray::surface::windows::handle_menu_event",
        None::<&str>
      );
    }
  }
}

fn handle_tray_event(
  app_handle: &AppHandle,
  state: &WindowsTrayState,
  event: TrayIconEvent,
) {
  if let TrayIconEvent::Click {
    position,
    rect,
    button: MouseButton::Left,
    button_state: MouseButtonState::Up,
    ..
  } = event
  {
    if has_visible_flyout_frame(state) {
      toggle_flyout_window(app_handle, state, position, rect);
    } else {
      hide_flyout_window(app_handle);
      crate::adapters::tray::restore_main_window(app_handle);
    }
  }
}

fn has_visible_flyout_frame(state: &WindowsTrayState) -> bool {
  state
    .latest_frame
    .lock()
    .unwrap()
    .as_ref()
    .is_some_and(|frame| !frame.items.is_empty())
}

fn toggle_flyout_window(
  app_handle: &AppHandle,
  state: &WindowsTrayState,
  click_position: PhysicalPosition<f64>,
  tray_rect: Rect,
) {
  let Some(window) = app_handle.get_webview_window(TRAY_WIDGET_FLYOUT_LABEL) else {
    log_warn!(
      "Windows tray flyout window was not available",
      "tray::surface::windows::toggle_flyout_window",
      None::<&str>
    );
    return;
  };

  if window.is_visible().unwrap_or(false) {
    if let Err(e) = window.hide() {
      log_warn!(
        &format!("failed to hide Windows tray flyout: {e}"),
        "tray::surface::windows::toggle_flyout_window",
        None::<&str>
      );
    }
    return;
  }

  let monitor = match app_handle.monitor_from_point(click_position.x, click_position.y) {
    Ok(monitor) => monitor,
    Err(e) => {
      log_warn!(
        &format!("failed to resolve monitor for Windows tray flyout: {e}"),
        "tray::surface::windows::toggle_flyout_window",
        None::<&str>
      );
      None
    }
  };
  let scale_factor = monitor
    .as_ref()
    .map(Monitor::scale_factor)
    .unwrap_or_else(|| window.scale_factor().unwrap_or(1.0));
  let flyout_size =
    PhysicalSize::new(FLYOUT_WIDTH * scale_factor, FLYOUT_HEIGHT * scale_factor);
  let position = flyout_position(
    physical_rect_from_tauri_rect(tray_rect, scale_factor),
    monitor.as_ref().map(monitor_work_area),
    flyout_size,
    FLYOUT_OFFSET * scale_factor,
  );

  if let Err(e) = window.set_position(position) {
    log_warn!(
      &format!("failed to position Windows tray flyout: {e}"),
      "tray::surface::windows::toggle_flyout_window",
      None::<&str>
    );
  }

  if let Some(frame) = state.latest_frame.lock().unwrap().clone() {
    emit_flyout_frame_to_window(&window, &frame);
  }

  if let Err(e) = window.show() {
    log_warn!(
      &format!("failed to show Windows tray flyout: {e}"),
      "tray::surface::windows::toggle_flyout_window",
      None::<&str>
    );
  }

  if let Err(e) = window.set_focus() {
    log_warn!(
      &format!("failed to focus Windows tray flyout: {e}"),
      "tray::surface::windows::toggle_flyout_window",
      None::<&str>
    );
  }
}

fn hide_flyout_window(app_handle: &AppHandle) {
  if let Some(window) = app_handle.get_webview_window(TRAY_WIDGET_FLYOUT_LABEL)
    && let Err(e) = window.hide()
  {
    log_warn!(
      &format!("failed to hide Windows tray flyout: {e}"),
      "tray::surface::windows::hide_flyout_window",
      None::<&str>
    );
  }
}

fn physical_rect_from_tauri_rect(rect: Rect, scale_factor: f64) -> PhysicalRectF {
  let position = rect.position.to_physical::<f64>(scale_factor);
  let size = rect.size.to_physical::<f64>(scale_factor);

  PhysicalRectF {
    x: position.x,
    y: position.y,
    width: size.width,
    height: size.height,
  }
}

fn monitor_work_area(monitor: &Monitor) -> PhysicalRectF {
  let work_area = monitor.work_area();

  PhysicalRectF {
    x: work_area.position.x as f64,
    y: work_area.position.y as f64,
    width: work_area.size.width as f64,
    height: work_area.size.height as f64,
  }
}

fn flyout_position(
  tray_rect: PhysicalRectF,
  work_area: Option<PhysicalRectF>,
  flyout_size: PhysicalSize<f64>,
  offset: f64,
) -> PhysicalPosition<f64> {
  let mut x = tray_rect.right() - flyout_size.width;
  let mut y = tray_rect.y - flyout_size.height - offset;

  if let Some(work_area) = work_area {
    if tray_rect.bottom() <= work_area.y {
      x = tray_rect.center_x() - flyout_size.width / 2.0;
      y = tray_rect.bottom() + offset;
    } else if tray_rect.right() <= work_area.x {
      x = tray_rect.right() + offset;
      y = tray_rect.center_y() - flyout_size.height / 2.0;
    } else if tray_rect.x >= work_area.right() {
      x = tray_rect.x - flyout_size.width - offset;
      y = tray_rect.center_y() - flyout_size.height / 2.0;
    }

    x = clamp_to_bounds(x, flyout_size.width, work_area.x, work_area.right());
    y = clamp_to_bounds(y, flyout_size.height, work_area.y, work_area.bottom());
  }

  PhysicalPosition::new(x.round(), y.round())
}

fn clamp_to_bounds(value: f64, size: f64, min: f64, max: f64) -> f64 {
  let max_start = max - size;

  if max_start < min {
    min
  } else {
    value.clamp(min, max_start)
  }
}

fn emit_flyout_frame(app_handle: &AppHandle, frame: &WindowsFlyoutFrame) {
  if let Some(window) = app_handle.get_webview_window(TRAY_WIDGET_FLYOUT_LABEL) {
    emit_flyout_frame_to_window(&window, frame);
  }
}

fn emit_flyout_frame_to_window(window: &WebviewWindow, frame: &WindowsFlyoutFrame) {
  if let Err(e) = window.emit(EVENT_TRAY_WIDGET_FRAME, frame) {
    log_warn!(
      &format!("failed to emit Windows tray flyout frame: {e}"),
      "tray::surface::windows::emit_flyout_frame_to_window",
      None::<&str>
    );
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowsFlyoutFrame {
  items: Vec<WindowsFlyoutItem>,
  tooltip: String,
}

impl WindowsFlyoutFrame {
  fn from_frame(frame: &TrayFrame) -> Self {
    Self {
      items: frame
        .items
        .iter()
        .map(WindowsFlyoutItem::from_item)
        .collect(),
      tooltip: frame.tooltip.clone(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowsFlyoutItem {
  metric: TrayMetric,
  value: String,
  state: &'static str,
}

impl WindowsFlyoutItem {
  fn from_item(item: &TrayFrameItem) -> Self {
    Self {
      metric: item.metric,
      value: item.value.clone(),
      state: state_name(item.state),
    }
  }
}

#[derive(Default)]
struct WindowsIconCache {
  images: HashMap<WindowsFrameKey, Image<'static>>,
  order: VecDeque<WindowsFrameKey>,
}

impl WindowsIconCache {
  fn image_for_frame(&mut self, frame: &TrayFrame) -> Image<'static> {
    let key = WindowsFrameKey::from_frame(frame);

    if let Some(image) = self.images.get(&key) {
      return image.clone();
    }

    let image = render_frame_icon(frame);

    if self.images.len() >= ICON_CACHE_LIMIT
      && let Some(oldest) = self.order.pop_front()
    {
      self.images.remove(&oldest);
    }

    self.order.push_back(key.clone());
    self.images.insert(key, image.clone());

    image
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WindowsFrameKey {
  state: MetricState,
}

impl WindowsFrameKey {
  fn from_frame(frame: &TrayFrame) -> Self {
    Self {
      state: worst_metric_state(&frame.items),
    }
  }
}

fn render_frame_icon(frame: &TrayFrame) -> Image<'static> {
  render_hv_outline_image(worst_metric_state(&frame.items))
}

fn render_hv_outline_image(state: MetricState) -> Image<'static> {
  let mut canvas = IconCanvas::new(ICON_SIZE, ICON_SIZE);
  draw_hv_outline_icon(&mut canvas, state_color(state));

  Image::new_owned(canvas.into_rgba(), ICON_SIZE, ICON_SIZE)
}

fn worst_metric_state(items: &[TrayFrameItem]) -> MetricState {
  items
    .iter()
    .map(|item| item.state)
    .max_by_key(|state| metric_state_rank(*state))
    .unwrap_or(MetricState::Normal)
}

fn state_color(state: MetricState) -> Rgba {
  match state {
    MetricState::Normal => ICON_COLOR,
    MetricState::Warning => WARNING_COLOR,
    MetricState::Critical => CRITICAL_COLOR,
  }
}

fn metric_state_rank(state: MetricState) -> u8 {
  match state {
    MetricState::Normal => 0,
    MetricState::Warning => 1,
    MetricState::Critical => 2,
  }
}

fn state_name(state: MetricState) -> &'static str {
  match state {
    MetricState::Normal => "normal",
    MetricState::Warning => "warning",
    MetricState::Critical => "critical",
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PhysicalRectF {
  x: f64,
  y: f64,
  width: f64,
  height: f64,
}

impl PhysicalRectF {
  fn right(self) -> f64 {
    self.x + self.width
  }

  fn bottom(self) -> f64 {
    self.y + self.height
  }

  fn center_x(self) -> f64 {
    self.x + self.width / 2.0
  }

  fn center_y(self) -> f64 {
    self.y + self.height / 2.0
  }
}

// This is the `assets/hv-icon-outline.svg` geometry in its 658.21 viewBox.
// Windows tray APIs require an HICON, so the SVG outline is rasterized here
// and tinted from the current worst metric state.
const SVG_VIEWBOX: f32 = 658.21;
const SVG_ICON_PADDING: f32 = 4.0;
const SVG_RECT: RectF = RectF {
  x: 36.55,
  y: 164.05,
  width: 585.81,
  height: 330.1,
};
const SVG_RECT_RADIUS: f32 = 25.0;
const SVG_RECT_STROKE_WIDTH: f32 = 25.0;
const SVG_WAVE_STROKE_WIDTH: f32 = 30.0;
const SVG_WAVE: [CubicBezier; 3] = [
  CubicBezier {
    p0: PointF {
      x: 622.26,
      y: 385.68,
    },
    p1: PointF {
      x: 524.51,
      y: 385.49,
    },
    p2: PointF {
      x: 524.73,
      y: 273.49,
    },
    p3: PointF {
      x: 426.98,
      y: 273.30,
    },
  },
  CubicBezier {
    p0: PointF {
      x: 426.98,
      y: 273.30,
    },
    p1: PointF {
      x: 329.22,
      y: 273.11,
    },
    p2: PointF {
      x: 329.01,
      y: 385.11,
    },
    p3: PointF {
      x: 231.25,
      y: 384.92,
    },
  },
  CubicBezier {
    p0: PointF {
      x: 231.25,
      y: 384.92,
    },
    p1: PointF {
      x: 133.49,
      y: 384.73,
    },
    p2: PointF {
      x: 133.71,
      y: 272.73,
    },
    p3: PointF {
      x: 35.96,
      y: 272.54,
    },
  },
];

fn draw_hv_outline_icon(canvas: &mut IconCanvas, color: Rgba) {
  let transform = SvgTransform::new();
  let rect = transform.rect(SVG_RECT);
  let rect_radius = transform.length(SVG_RECT_RADIUS);
  let rect_stroke = transform.length(SVG_RECT_STROKE_WIDTH).max(4.0);
  let wave_stroke = transform.length(SVG_WAVE_STROKE_WIDTH).max(5.0);

  canvas.draw_rounded_rect_stroke(rect, rect_radius, rect_stroke, color);

  for curve in SVG_WAVE {
    draw_cubic_stroke(canvas, transform.cubic(curve), wave_stroke, color);
  }
}

fn draw_cubic_stroke(
  canvas: &mut IconCanvas,
  curve: CubicBezier,
  stroke_width: f32,
  color: Rgba,
) {
  let mut previous = curve.p0;

  for step in 1..=72 {
    let t = step as f32 / 72.0;
    let next = curve.sample(t);
    canvas.draw_stroked_segment(previous, next, stroke_width, color);
    previous = next;
  }
}

#[derive(Clone, Copy, Debug)]
struct SvgTransform {
  scale: f32,
}

impl SvgTransform {
  fn new() -> Self {
    Self {
      scale: (ICON_SIZE as f32 - SVG_ICON_PADDING * 2.0) / SVG_VIEWBOX,
    }
  }

  fn point(&self, point: PointF) -> PointF {
    PointF {
      x: point.x * self.scale + SVG_ICON_PADDING,
      y: point.y * self.scale + SVG_ICON_PADDING,
    }
  }

  fn length(&self, value: f32) -> f32 {
    value * self.scale
  }

  fn rect(&self, rect: RectF) -> RectF {
    RectF {
      x: rect.x * self.scale + SVG_ICON_PADDING,
      y: rect.y * self.scale + SVG_ICON_PADDING,
      width: rect.width * self.scale,
      height: rect.height * self.scale,
    }
  }

  fn cubic(&self, cubic: CubicBezier) -> CubicBezier {
    CubicBezier {
      p0: self.point(cubic.p0),
      p1: self.point(cubic.p1),
      p2: self.point(cubic.p2),
      p3: self.point(cubic.p3),
    }
  }
}

#[derive(Clone, Copy, Debug)]
struct PointF {
  x: f32,
  y: f32,
}

#[derive(Clone, Copy, Debug)]
struct RectF {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

#[derive(Clone, Copy, Debug)]
struct CubicBezier {
  p0: PointF,
  p1: PointF,
  p2: PointF,
  p3: PointF,
}

impl CubicBezier {
  fn sample(&self, t: f32) -> PointF {
    let one_minus_t = 1.0 - t;
    let a = one_minus_t * one_minus_t * one_minus_t;
    let b = 3.0 * one_minus_t * one_minus_t * t;
    let c = 3.0 * one_minus_t * t * t;
    let d = t * t * t;

    PointF {
      x: self.p0.x * a + self.p1.x * b + self.p2.x * c + self.p3.x * d,
      y: self.p0.y * a + self.p1.y * b + self.p2.y * c + self.p3.y * d,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgba([u8; 4]);

impl Rgba {
  fn with_alpha_factor(self, factor: f32) -> Self {
    let mut color = self;
    color.0[3] = (color.0[3] as f32 * factor.clamp(0.0, 1.0)).round() as u8;
    color
  }
}

struct IconCanvas {
  width: u32,
  height: u32,
  pixels: Vec<u8>,
}

impl IconCanvas {
  fn new(width: u32, height: u32) -> Self {
    Self {
      width,
      height,
      pixels: vec![0; (width * height * 4) as usize],
    }
  }

  fn into_rgba(self) -> Vec<u8> {
    self.pixels
  }

  fn draw_rounded_rect_stroke(
    &mut self,
    rect: RectF,
    radius: f32,
    stroke_width: f32,
    color: Rgba,
  ) {
    let padding = stroke_width / 2.0 + 2.0;
    let left = (rect.x - padding).floor() as i32;
    let top = (rect.y - padding).floor() as i32;
    let right = (rect.x + rect.width + padding).ceil() as i32;
    let bottom = (rect.y + rect.height + padding).ceil() as i32;

    for y in top..=bottom {
      for x in left..=right {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        let distance = rounded_rect_signed_distance(px, py, rect, radius).abs();
        let coverage = stroke_coverage(distance, stroke_width);

        if coverage > 0.0 {
          self.blend_pixel(x, y, color.with_alpha_factor(coverage));
        }
      }
    }
  }

  fn draw_stroked_segment(
    &mut self,
    start: PointF,
    end: PointF,
    stroke_width: f32,
    color: Rgba,
  ) {
    let padding = stroke_width / 2.0 + 2.0;
    let left = (start.x.min(end.x) - padding).floor() as i32;
    let top = (start.y.min(end.y) - padding).floor() as i32;
    let right = (start.x.max(end.x) + padding).ceil() as i32;
    let bottom = (start.y.max(end.y) + padding).ceil() as i32;

    for y in top..=bottom {
      for x in left..=right {
        let point = PointF {
          x: x as f32 + 0.5,
          y: y as f32 + 0.5,
        };
        let distance = distance_to_segment(point, start, end);
        let coverage = stroke_coverage(distance, stroke_width);

        if coverage > 0.0 {
          self.blend_pixel(x, y, color.with_alpha_factor(coverage));
        }
      }
    }
  }

  fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba) {
    if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
      return;
    }

    let index = ((y as u32 * self.width + x as u32) * 4) as usize;
    let src = color.0;
    let src_alpha = src[3] as u32;
    let dst_alpha = self.pixels[index + 3] as u32;
    let out_alpha = src_alpha + (dst_alpha * (255 - src_alpha)) / 255;

    if out_alpha == 0 {
      return;
    }

    for (channel, src_channel) in src.iter().take(3).enumerate() {
      let dst_channel = self.pixels[index + channel] as u32;
      let out_channel = (*src_channel as u32 * src_alpha
        + dst_channel * dst_alpha * (255 - src_alpha) / 255)
        / out_alpha;
      self.pixels[index + channel] = out_channel as u8;
    }
    self.pixels[index + 3] = out_alpha as u8;
  }
}

fn stroke_coverage(distance: f32, stroke_width: f32) -> f32 {
  let half_width = stroke_width / 2.0;
  (half_width + 0.75 - distance).clamp(0.0, 1.0)
}

fn distance_to_segment(point: PointF, start: PointF, end: PointF) -> f32 {
  let vx = end.x - start.x;
  let vy = end.y - start.y;
  let wx = point.x - start.x;
  let wy = point.y - start.y;
  let len_sq = vx * vx + vy * vy;

  if len_sq <= f32::EPSILON {
    return ((point.x - start.x).powi(2) + (point.y - start.y).powi(2)).sqrt();
  }

  let t = ((wx * vx + wy * vy) / len_sq).clamp(0.0, 1.0);
  let projection = PointF {
    x: start.x + vx * t,
    y: start.y + vy * t,
  };

  ((point.x - projection.x).powi(2) + (point.y - projection.y).powi(2)).sqrt()
}

fn rounded_rect_signed_distance(px: f32, py: f32, rect: RectF, radius: f32) -> f32 {
  let half_width = rect.width / 2.0;
  let half_height = rect.height / 2.0;
  let center_x = rect.x + half_width;
  let center_y = rect.y + half_height;
  let qx = (px - center_x).abs() - (half_width - radius);
  let qy = (py - center_y).abs() - (half_height - radius);
  let outside_x = qx.max(0.0);
  let outside_y = qy.max(0.0);
  let outside = (outside_x * outside_x + outside_y * outside_y).sqrt();
  let inside = qx.max(qy).min(0.0);

  outside + inside - radius
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tray::widget::TrayMetricIcon;

  fn item(metric: TrayMetric, value: &str, state: MetricState) -> TrayFrameItem {
    TrayFrameItem {
      metric,
      icon: TrayMetricIcon {
        fallback_label: "CPU",
        macos_symbol_name: "cpu",
        accessibility_label: "CPU usage",
      },
      value: value.to_string(),
      state,
    }
  }

  #[test]
  fn renders_non_empty_rgba_icon_for_metric_frame() {
    let frame = TrayFrame {
      title: Some("CPU 42%  GPU 91%".to_string()),
      items: vec![
        item(TrayMetric::Cpu, "42%", MetricState::Normal),
        item(TrayMetric::Gpu, "91%", MetricState::Critical),
      ],
      tooltip: "CPU usage: 42% (normal)\nGPU usage: 91% (critical)".to_string(),
    };

    let image = render_frame_icon(&frame);

    assert_eq!(image.width(), ICON_SIZE);
    assert_eq!(image.height(), ICON_SIZE);
    assert_eq!(image.rgba().len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
    assert!(image.rgba().chunks_exact(4).any(|pixel| pixel[3] > 0));
  }

  #[test]
  fn cache_key_tracks_rendered_metric_and_state() {
    let first = TrayFrame {
      title: Some("CPU 69%".to_string()),
      items: vec![item(TrayMetric::Cpu, "69%", MetricState::Normal)],
      tooltip: "first".to_string(),
    };
    let same_rendered_icon = TrayFrame {
      title: Some("CPU changed title".to_string()),
      items: vec![item(TrayMetric::Cpu, "70%", MetricState::Normal)],
      tooltip: "second".to_string(),
    };
    let warning = TrayFrame {
      title: Some("CPU 70%".to_string()),
      items: vec![item(TrayMetric::Cpu, "70%", MetricState::Warning)],
      tooltip: "third".to_string(),
    };

    assert_eq!(
      WindowsFrameKey::from_frame(&first),
      WindowsFrameKey::from_frame(&same_rendered_icon)
    );
    assert_ne!(
      WindowsFrameKey::from_frame(&first),
      WindowsFrameKey::from_frame(&warning)
    );
  }

  #[test]
  fn warning_and_critical_states_use_state_colors() {
    assert_eq!(state_color(MetricState::Normal), ICON_COLOR);
    assert_eq!(state_color(MetricState::Warning), WARNING_COLOR);
    assert_eq!(state_color(MetricState::Critical), CRITICAL_COLOR);
  }

  #[test]
  fn icon_tint_follows_worst_metric_state() {
    let frame = TrayFrame {
      title: Some("CPU 42%  GPU 91%".to_string()),
      items: vec![
        item(TrayMetric::Cpu, "42%", MetricState::Normal),
        item(TrayMetric::Gpu, "91%", MetricState::Critical),
      ],
      tooltip: "CPU usage: 42% (normal)\nGPU usage: 91% (critical)".to_string(),
    };

    let image = render_frame_icon(&frame);

    assert!(
      image
        .rgba()
        .chunks_exact(4)
        .any(|pixel| pixel[0..3] == CRITICAL_COLOR.0[0..3] && pixel[3] > 0)
    );
  }

  #[test]
  fn empty_frame_uses_normal_hv_outline_tint() {
    let frame = TrayFrame {
      title: None,
      items: vec![],
      tooltip: "HardwareVisualizer".to_string(),
    };

    let image = render_frame_icon(&frame);

    assert!(
      image
        .rgba()
        .chunks_exact(4)
        .any(|pixel| pixel[0..3] == ICON_COLOR.0[0..3] && pixel[3] > 0)
    );
  }

  #[test]
  fn flyout_frame_preserves_metric_values_for_readable_ui() {
    let frame = TrayFrame {
      title: Some("CPU 42%  GPU 91%".to_string()),
      items: vec![
        item(TrayMetric::Cpu, "42%", MetricState::Normal),
        item(TrayMetric::Gpu, "91%", MetricState::Critical),
      ],
      tooltip: "CPU usage: 42% (normal)\nGPU usage: 91% (critical)".to_string(),
    };

    let flyout_frame = WindowsFlyoutFrame::from_frame(&frame);

    assert_eq!(flyout_frame.tooltip, frame.tooltip);
    assert_eq!(flyout_frame.items[0].value, "42%");
    assert_eq!(flyout_frame.items[0].state, "normal");
    assert_eq!(flyout_frame.items[1].value, "91%");
    assert_eq!(flyout_frame.items[1].state, "critical");
  }

  #[test]
  fn flyout_position_opens_above_a_bottom_taskbar_tray_rect() {
    let position = flyout_position(
      PhysicalRectF {
        x: 1880.0,
        y: 1048.0,
        width: 32.0,
        height: 32.0,
      },
      Some(PhysicalRectF {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1040.0,
      }),
      PhysicalSize::new(300.0, 148.0),
      12.0,
    );

    assert_eq!(position.x, 1612.0);
    assert_eq!(position.y, 888.0);
  }

  #[test]
  fn flyout_position_does_not_clamp_unknown_monitor_to_zero() {
    let position = flyout_position(
      PhysicalRectF {
        x: -80.0,
        y: 1048.0,
        width: 32.0,
        height: 32.0,
      },
      None,
      PhysicalSize::new(300.0, 148.0),
      12.0,
    );

    assert_eq!(position.x, -348.0);
    assert_eq!(position.y, 888.0);
  }

  #[test]
  fn flyout_position_opens_below_a_top_taskbar_tray_rect() {
    let position = flyout_position(
      PhysicalRectF {
        x: 944.0,
        y: 0.0,
        width: 32.0,
        height: 32.0,
      },
      Some(PhysicalRectF {
        x: 0.0,
        y: 40.0,
        width: 1920.0,
        height: 1040.0,
      }),
      PhysicalSize::new(300.0, 148.0),
      12.0,
    );

    assert_eq!(position.x, 810.0);
    assert_eq!(position.y, 44.0);
  }
}

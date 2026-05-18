/*
 * Reusable UI components and drawing helpers.
 * Includes logic for legend entries (standard and colorblind-friendly)
 *   and custom line styling (dashed, dotted, dash-dot patterns).
 */

use eframe::egui;
use egui::{Color32, Stroke};

// Draw a colored legend entry.
pub fn draw_legend_entry(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, color);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}

#[derive(Debug, Clone, Copy)]
pub enum LegendStyle {
    Crosshatch,
    Dots,
    SolidLine,
    LongDash,
    ShortDash,
    DottedLine,
    DashDotLine,
    DashDotDotLine,
    Label(&'static str),
    LabelChar(char),
}

// Draw a patterned legend entry.
pub fn draw_legend_entry_colorblind(ui: &mut egui::Ui, label: &str, style: LegendStyle) {
    ui.horizontal(|ui| {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, Color32::from_gray(220));
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Middle,
        );

        match style {
            LegendStyle::Crosshatch => {
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_bottom()],
                    Stroke::new(1.2, Color32::BLACK),
                );
                ui.painter().line_segment(
                    [rect.right_top(), rect.left_bottom()],
                    Stroke::new(1.2, Color32::BLACK),
                );
            }
            LegendStyle::Dots => {
                ui.painter()
                    .circle_filled(rect.center(), 2.0, Color32::BLACK);
                ui.painter().circle_filled(
                    egui::pos2(rect.center().x - 4.0, rect.center().y),
                    1.4,
                    Color32::BLACK,
                );
                ui.painter().circle_filled(
                    egui::pos2(rect.center().x + 4.0, rect.center().y),
                    1.4,
                    Color32::BLACK,
                );
            }
            LegendStyle::SolidLine => {
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left() + 1.0, rect.center().y),
                        egui::pos2(rect.right() - 1.0, rect.center().y),
                    ],
                    Stroke::new(2.0, Color32::BLACK),
                );
            }
            LegendStyle::Label(prefix) => {
                ui.painter()
                    .circle_filled(rect.center(), 6.0, Color32::from_gray(180));
                ui.painter()
                    .circle_stroke(rect.center(), 6.0, Stroke::new(1.0, Color32::BLACK));
                let font_size = 10.0;
                let font_id = egui::FontId::monospace(font_size);
                let text_width = prefix.len() as f32 * font_size * 0.6;
                let text_height = font_size * 0.8;
                let text_pos = egui::pos2(
                    rect.center().x - text_width / 2.0,
                    rect.center().y - text_height / 2.0 + font_size / 5.0,
                );
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    prefix,
                    font_id,
                    Color32::BLACK,
                );
            }
            LegendStyle::LabelChar(character) => {
                ui.painter()
                    .circle_filled(rect.center(), 6.0, Color32::from_gray(180));
                ui.painter()
                    .circle_stroke(rect.center(), 6.0, Stroke::new(1.0, Color32::BLACK));
                let font_size = 10.0;
                let font_id = egui::FontId::monospace(font_size);
                let text_width = font_size * 0.6;
                let text_height = font_size * 0.8;
                let text_pos = egui::pos2(
                    rect.center().x - text_width / 2.0,
                    rect.center().y - text_height / 2.0 + font_size / 5.0,
                );
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    character,
                    font_id,
                    Color32::BLACK,
                );
            }
            LegendStyle::LongDash => draw_dashed_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
                6.0,
                3.0,
            ),
            LegendStyle::ShortDash => draw_dashed_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
                3.0,
                2.0,
            ),
            LegendStyle::DottedLine => draw_dotted_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
            LegendStyle::DashDotLine => draw_dash_dot_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
            LegendStyle::DashDotDotLine => draw_dash_dot_dot_line(
                ui.painter(),
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
                Stroke::new(2.0, Color32::BLACK),
            ),
        }

        ui.add_space(6.0);
        ui.label(label);
    });
    ui.add_space(4.0);
}

// Draw a dashed line.
pub fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let mut dist = 0.0_f32;
    while dist < len {
        let seg_start = dist;
        let seg_end = (dist + dash_len).min(len);
        let p0 = egui::pos2(from.x + ux * seg_start, from.y + uy * seg_start);
        let p1 = egui::pos2(from.x + ux * seg_end, from.y + uy * seg_end);
        painter.line_segment([p0, p1], stroke);
        dist += dash_len + gap_len;
    }
}

// Draw a dotted line.
pub fn draw_dotted_line(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let step = 5.0_f32;
    let radius = (stroke.width * 0.35).max(1.0);

    let mut dist = 0.0_f32;
    while dist <= len {
        let point = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(point, radius, stroke.color);
        dist += step;
    }
}

pub fn draw_dash_dot_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot, (stroke.width * 0.35).max(1.0), stroke.color);
        dist += 4.0;
    }
}

pub fn draw_dash_dot_dot_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: Stroke,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;
    let radius = (stroke.width * 0.35).max(1.0);
    let mut dist = 0.0_f32;
    while dist < len {
        let dash_end = (dist + 7.0).min(len);
        let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        let p1 = egui::pos2(from.x + ux * dash_end, from.y + uy * dash_end);
        painter.line_segment([p0, p1], stroke);
        dist = dash_end + 3.0;
        if dist >= len {
            break;
        }
        let dot1 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot1, radius, stroke.color);
        dist += 3.0;
        if dist >= len {
            break;
        }
        let dot2 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
        painter.circle_filled(dot2, radius, stroke.color);
        dist += 4.0;
    }
}

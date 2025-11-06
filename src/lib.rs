#![feature(min_specialization)]
use egui::{Color32, FontFamily, FontId, Galley, Rect, Sense, Stroke, pos2, vec2};
use egui_plot::{Plot, PlotResponse, PlotUi};
use std::{fmt::Display, sync::Arc};

pub trait EnumPlottable: PartialEq {
    fn display(&self) -> String;
}

impl<T> EnumPlottable for T
where
    T: Display + PartialEq,
{
    default fn display(&self) -> String {
        format!("{}", self)
    }
}

#[cfg(feature = "fancier_float_formatting")]
impl EnumPlottable for f64 {
    fn display(&self) -> String {
        let s = format!("{:.3}", self);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(feature = "fancier_float_formatting")]
impl EnumPlottable for f32 {
    fn display(&self) -> String {
        let s = format!("{:.3}", self);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(feature = "debug_impl")]
impl<T> EnumPlottable for T
where
    T: Debug + Eq,
{
    default fn display(&self) -> String {
        format!("{}", self)
    }
}

pub struct EnumPlotLine<T: EnumPlottable> {
    points: Vec<(f64, T)>,
}

impl<T: EnumPlottable> EnumPlotLine<T> {
    pub fn new(points: Vec<(f64, T)>) -> Self {
        EnumPlotLine { points }
    }
}

//todo swap this to be an iter like thing to avoid a ton of allocs
pub trait EnumPlotLineTrait {
    fn get_edges_and_labels(&self) -> Vec<(f64, String)>;
}

impl<T: EnumPlottable> EnumPlotLineTrait for EnumPlotLine<T> {
    fn get_edges_and_labels(&self) -> Vec<(f64, String)> {
        //the mins and maxes are needed for the painter to work properly
        let mut out_vec = vec![(f32::MIN as f64, self.points.first().unwrap().1.display())];
        out_vec.extend(
            self.points
                .windows(2)
                .filter_map(|e| (e[0].1 != e[1].1).then(|| (e[0].0, e[1].1.display()))),
        );
        out_vec.push((f32::MAX as f64, self.points.last().unwrap().1.display()));
        out_vec
    }
}

pub struct EnumPlotUiStyle {
    pub line_height: f32,
    pub line_spacing: f32,
    pub margin: f32,
    pub text_height: f32,
    pub text_color: Color32,
    pub transition_len: f32,
    pub line_style: Stroke,
    pub side_margin: f32,
    pub hover_text: bool,
}

impl EnumPlotUiStyle {
    fn new(ui: &mut egui::Ui) -> Self {
        EnumPlotUiStyle {
            line_height: 25.0,
            line_spacing: 10.0,
            margin: 5.0,
            transition_len: 0.5,
            line_style: ui.ctx().style().visuals.noninteractive().fg_stroke,
            text_height: 15.0,
            text_color: ui.ctx().style().noninteractive().text_color(),
            side_margin: 75.0,
            hover_text: false,
        }
    }
}

pub struct EnumPlotUi {
    names: Vec<String>,
    lines: Vec<Box<dyn EnumPlotLineTrait>>,
}

impl EnumPlotUi {
    pub fn add_line<T>(&mut self, name: String, plot_line: EnumPlotLine<T>)
    where
        T: EnumPlottable + 'static,
    {
        self.names.push(name);
        self.lines.push(Box::new(plot_line));
    }

    pub fn add_custom_plot(&mut self, name: String, plottable: impl EnumPlotLineTrait + 'static) {
        self.names.push(name);
        self.lines.push(Box::new(plottable));
    }
}

pub struct EnumPlot {
    pub style: EnumPlotUiStyle,
    enum_plot_ui: Option<EnumPlotUi>,
}

impl EnumPlot {
    pub fn new(ui: &mut egui::Ui) -> Self {
        EnumPlot {
            style: EnumPlotUiStyle::new(ui),
            enum_plot_ui: None,
        }
    }

    pub fn setup_enum_plot(&mut self, enum_build_fn: impl FnOnce(&mut EnumPlotUi)) {
        let mut enum_plot_ui = EnumPlotUi {
            lines: Vec::new(),
            names: Vec::new(),
        };
        enum_build_fn(&mut enum_plot_ui);
        self.enum_plot_ui = Some(enum_plot_ui);
    }
    pub fn show<R>(
        &self,
        ui: &mut egui::Ui,
        plot: Plot,
        plot_build_fn: impl FnOnce(&mut PlotUi) -> R,
    ) -> PlotResponse<R> {
        if let Some(enum_plot_ui) = &self.enum_plot_ui {
            let line_count = enum_plot_ui.lines.len();
            let vertical_space_needed = line_count as f32 * self.style.line_height
                + line_count.saturating_sub(1) as f32 * self.style.line_spacing
                + 2.0 * self.style.margin;
            let plot_height = ui.available_height() - vertical_space_needed;
            let plot = plot.height(plot_height);
            let plot_ret = plot.show(ui, plot_build_fn);

            let id = FontId {
                size: self.style.text_height,
                family: FontFamily::default(),
            };

            //todo make hover work nicely
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
            for (i, line) in enum_plot_ui.lines.iter().enumerate() {
                let edges = line.get_edges_and_labels();
                //todo add screen occlusion checking here, to avoid rendering stuff far off screen
                let top_y_val = response.rect.top()
                    + self.style.margin
                    + (self.style.line_height + self.style.line_spacing) * i as f32;
                for (j, edges) in edges.windows(2).enumerate() {
                    let x_val_start = plot_ret.transform.position_from_point_x(edges[0].0);
                    let x_val_end = plot_ret.transform.position_from_point_x(edges[1].0);
                    let mut x_val_start_clip = ui.clip_rect().x_range();
                    x_val_start_clip.min += self.style.side_margin;
                    let x_val_start_clipped = x_val_start_clip.clamp(x_val_start);
                    let x_val_end_clipped = ui.clip_rect().x_range().clamp(x_val_end);

                    let cross_x_offset =
                        (self.style.transition_len / 100.0 * ui.clip_rect().x_range().span() / 2.0)
                            .min((x_val_end_clipped - x_val_start_clipped) * 0.5)
                            / 2.0;
                    if j != 0 {
                        painter.line_segment(
                            [
                                pos2(x_val_start_clipped + cross_x_offset, top_y_val),
                                pos2(
                                    x_val_start_clipped,
                                    top_y_val + self.style.line_height / 2.0,
                                ),
                            ],
                            self.style.line_style,
                        );
                        painter.line_segment(
                            [
                                pos2(
                                    x_val_start_clipped + cross_x_offset,
                                    top_y_val + self.style.line_height,
                                ),
                                pos2(
                                    x_val_start_clipped,
                                    top_y_val + self.style.line_height / 2.0,
                                ),
                            ],
                            self.style.line_style,
                        );
                    }
                    painter.line_segment(
                        [
                            pos2(cross_x_offset + x_val_start_clipped, top_y_val),
                            pos2(x_val_end_clipped - cross_x_offset, top_y_val),
                        ],
                        self.style.line_style,
                    );
                    painter.line_segment(
                        [
                            pos2(
                                cross_x_offset + x_val_start_clipped,
                                top_y_val + self.style.line_height,
                            ),
                            pos2(
                                x_val_end_clipped - cross_x_offset,
                                top_y_val + self.style.line_height,
                            ),
                        ],
                        self.style.line_style,
                    );

                    painter.line_segment(
                        [
                            pos2(x_val_end_clipped - cross_x_offset, top_y_val),
                            pos2(x_val_end_clipped, top_y_val + self.style.line_height / 2.0),
                        ],
                        self.style.line_style,
                    );
                    painter.line_segment(
                        [
                            pos2(
                                x_val_end_clipped - cross_x_offset,
                                top_y_val + self.style.line_height,
                            ),
                            pos2(x_val_end_clipped, top_y_val + self.style.line_height / 2.0),
                        ],
                        self.style.line_style,
                    );

                    let text_pos = pos2(
                        (x_val_start_clipped + x_val_end_clipped) / 2.0,
                        top_y_val + self.style.line_height / 2.0,
                    );
                    let max_text_bb = Rect::from_center_size(
                        text_pos,
                        vec2(
                            x_val_end_clipped - x_val_start_clipped - cross_x_offset,
                            self.style.line_height,
                        ),
                    );
                    if self.style.hover_text {
                        let res = ui.allocate_rect(max_text_bb, Sense::hover());
                        res.on_hover_text_at_pointer(&edges[0].1.clone());
                    }
                    if let Some(galley) = best_fit_font(
                        ui.ctx(),
                        &edges[0].1.clone(),
                        max_text_bb,
                        1.0,
                        self.style.text_color,
                        &id,
                    ) {
                        painter.galley(
                            text_pos - galley.rect.center().to_vec2(),
                            galley,
                            self.style.text_color,
                        );
                    }
                }
            }
            let mut line_label_rect = ui.clip_rect();
            line_label_rect.set_right(line_label_rect.left() + self.style.side_margin);
            //hacky hacky bullshit that should get fixed when we cull smarter
            //for now just pencil over the fuckery
            //todo defuckme
            painter.rect_filled(line_label_rect, 0.0, ui.ctx().style().visuals.panel_fill);
            for (i, name) in enum_plot_ui.names.iter().enumerate() {
                let top_y_val = response.rect.top()
                    + self.style.margin
                    + (self.style.line_height + self.style.line_spacing) * i as f32;
                let max_label_bb = Rect::from_center_size(
                    pos2(
                        ui.clip_rect().left() + self.style.side_margin / 2.0,
                        top_y_val + self.style.line_height * 0.5,
                    ),
                    vec2(self.style.side_margin - 5.0, self.style.line_height),
                );
                if let Some(galley) = best_fit_font(
                    ui.ctx(),
                    name,
                    max_label_bb,
                    1.0,
                    self.style.text_color,
                    &id,
                ) {
                    painter.galley(
                        max_label_bb.center() - galley.rect.center().to_vec2(),
                        galley,
                        self.style.text_color,
                    );
                }
            }

            plot_ret
        } else {
            plot.show(ui, plot_build_fn)
        }
    }
}

fn best_fit_font(
    ctx: &egui::Context,
    text: &str,
    rect: Rect,
    min_size_pt: f32,
    color: Color32,
    font: &FontId,
) -> Option<Arc<Galley>> {
    let mut id = font.clone();
    loop {
        let galley = ctx.fonts_mut(|f| {
            // wrap_width == available width:
            f.layout_no_wrap(text.to_owned(), id.clone(), color)
        });

        if galley.size().x <= rect.width() && galley.size().y <= rect.height() {
            return Some(galley);
        }
        id.size -= 2.5;
        if id.size < min_size_pt {
            return None;
        }
    }
}

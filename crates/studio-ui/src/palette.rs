use crate::help::Help;
use crate::pro::{CHROME, FAINT, INK, INK_STRONG, LINE, MUTED};
use egui::{Align2, Color32, Context, CornerRadius, Frame, Key, KeyboardShortcut, Margin, Modifiers, RichText, Stroke};
use serde::{Deserialize, Serialize};

pub struct Cmd {
    pub label: String,
    pub hint: String,
    pub id: String,
}

impl Cmd {
    pub fn new(label: impl Into<String>, hint: impl Into<String>, id: impl Into<String>) -> Self {
        Cmd { label: label.into(), hint: hint.into(), id: id.into() }
    }
}

pub const ACCENTS: [(&str, [u8; 3]); 7] = [("Bleu", [10, 132, 255]), ("Violet", [140, 90, 255]), ("Rose", [255, 70, 140]), ("Orange", [255, 140, 30]), ("Vert", [48, 190, 90]), ("Turquoise", [20, 190, 200]), ("Graphite", [140, 140, 150])];
pub const SIZES: [f32; 6] = [0.85, 0.9, 1.0, 1.1, 1.25, 1.5];

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub zoom: f32,
    pub accent: usize,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { zoom: 1.0, accent: 0 }
    }
}

fn prefs_path() -> std::path::PathBuf {
    studio_core::data_dir().join("interface.json")
}

pub fn load_prefs() -> Prefs {
    std::fs::read(prefs_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn apply_prefs(ctx: &Context, p: Prefs) {
    ctx.set_zoom_factor(p.zoom.clamp(0.7, 2.0));
    let c = ACCENTS[p.accent.min(ACCENTS.len() - 1)].1;
    let a = Color32::from_rgb(c[0], c[1], c[2]);
    ctx.all_styles_mut(|s| {
        s.visuals.selection.bg_fill = a.gamma_multiply(0.55);
        s.visuals.selection.stroke.color = a;
        s.visuals.hyperlink_color = a;
        s.visuals.widgets.active.bg_fill = a;
        s.visuals.text_cursor.stroke.color = a;
    });
}

fn save_prefs(p: Prefs) {
    let _ = std::fs::create_dir_all(studio_core::data_dir());
    if let Ok(t) = serde_json::to_string_pretty(&p) {
        let _ = std::fs::write(prefs_path(), t);
    }
}

pub fn effect_cmds() -> Vec<Cmd> {
    studio_fx::stack::groups().into_iter().flat_map(|(g, ks)| ks.iter().map(move |k| Cmd::new(format!("Effet : {}", k.title()), g, format!("fx:{k:?}")))).collect()
}

const EFFECT_LEVELS: [u8; 20] = [5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95, 100];

/// Three thousand searchable, executable variants of the existing effects.
/// The vector is built once because command palettes repaint while they are open.
pub fn effect_variant_cmds() -> &'static [Cmd] {
    static OPTIONS: std::sync::OnceLock<Vec<Cmd>> = std::sync::OnceLock::new();
    OPTIONS.get_or_init(|| {
        let mut seen = std::collections::HashSet::new();
        studio_fx::stack::groups()
            .into_iter()
            .flat_map(|(group, kinds)| kinds.iter().map(move |kind| (group, *kind)))
            .filter(|(_, kind)| seen.insert(format!("{kind:?}")))
            .flat_map(|(group, kind)| {
                EFFECT_LEVELS.into_iter().map(move |amount| {
                    Cmd::new(
                        format!("Option créative : {} · {amount} %", kind.title()),
                        format!("{group} · intensité {amount} %"),
                        format!("fxv:{kind:?}:{amount}"),
                    )
                })
            })
            .take(3_000)
            .collect()
    })
}

/// Decodes both the original `fx:` commands and the intensity variants.
pub fn effect_from_command(id: &str) -> Option<studio_fx::stack::Fx> {
    if let Some(kind) = id.strip_prefix("fx:").and_then(studio_fx::stack::kind) {
        return Some(studio_fx::stack::Fx::new(kind));
    }
    let (kind, amount) = id.strip_prefix("fxv:")?.rsplit_once(':')?;
    let amount = amount.parse::<u8>().ok()?;
    if !EFFECT_LEVELS.contains(&amount) {
        return None;
    }
    let mut effect = studio_fx::stack::Fx::new(studio_fx::stack::kind(kind)?);
    effect.amount = amount as f32;
    Some(effect)
}

pub fn send_cmds(targets: &[(&str, &str)]) -> Vec<Cmd> {
    targets.iter().map(|(id, what)| Cmd::new(format!("Envoyer vers {}", studio_core::handoff::name(id)), *what, format!("send:{id}"))).collect()
}

fn command_icon(id: &str) -> &'static str {
    if id.starts_with("fx:") || id.starts_with("fxv:") {
        "✦"
    } else if id.starts_with("send:") {
        "↗"
    } else if id.starts_with("help:") {
        "?"
    } else if id.starts_with("ui-") {
        "◐"
    } else {
        "◆"
    }
}

fn include_effect_variants(query: &str) -> bool {
    !query.trim().is_empty()
}

pub struct Palette {
    pub open: bool,
    query: String,
    sel: usize,
    prefs: Prefs,
}

impl Palette {
    pub fn new(ctx: &Context) -> Self {
        let prefs = load_prefs();
        apply_prefs(ctx, prefs);
        Palette { open: false, query: String::new(), sel: 0, prefs }
    }

    pub fn keys(&mut self, ctx: &Context) {
        let k = ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::K)) || i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::P)));
        if k {
            self.open = !self.open;
            self.query.clear();
            self.sel = 0;
        }
    }

    fn builtin(&self, help: &Help) -> Vec<Cmd> {
        let mut v: Vec<Cmd> = help.titles().into_iter().map(|t| Cmd::new(format!("Aide : {t}"), "Tutoriel", format!("help:{t}"))).collect();
        v.push(Cmd::new("Ouvrir l'aide et l'assistant IA", "F1", "help:"));
        for z in SIZES {
            v.push(Cmd::new(format!("Interface : taille {:.0} %", z * 100.0), "Agrandit ou réduit toute l'interface", format!("ui-zoom:{z}")));
        }
        for (k, (n, _)) in ACCENTS.iter().enumerate() {
            v.push(Cmd::new(format!("Interface : couleur d'accent {n}"), "Sélections, curseurs actifs et liens", format!("ui-accent:{k}")));
        }
        v
    }

    pub fn show(&mut self, ctx: &Context, help: &mut Help, app: Vec<Cmd>) -> Option<String> {
        if !self.open {
            return None;
        }
        let builtin = self.builtin(help);
        let words: Vec<String> = self.query.to_lowercase().split_whitespace().map(str::to_string).collect();
        let variants = if include_effect_variants(&self.query) { effect_variant_cmds() } else { &[] };
        let hits: Vec<&Cmd> = app.iter().chain(&builtin).chain(variants).filter(|c| {
            let hay = format!("{} {}", c.label, c.hint).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        }).collect();
        let visible = hits.len().min(200);
        self.sel = self.sel.min(visible.saturating_sub(1));
        let (up, down, enter, esc) = ctx.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown), i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        if down {
            self.sel = (self.sel + 1).min(visible.saturating_sub(1));
        }
        if up {
            self.sel = self.sel.saturating_sub(1);
        }
        let mut chosen = if enter { hits.get(self.sel).map(|c| c.id.clone()) } else { None };
        let close = esc;
        egui::Area::new(egui::Id::NULL.with("palette")).order(egui::Order::Foreground).anchor(Align2::CENTER_TOP, [0.0, 80.0]).show(ctx, |ui| {
            Frame::new().fill(CHROME).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(14)).inner_margin(Margin::same(10)).shadow(egui::epaint::Shadow { offset: [0, 16], blur: 40, spread: 0, color: Color32::from_black_alpha(170) }).show(ui, |ui| {
                ui.set_width(560.0);
                let r = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Rechercher une commande, un effet, un tutoriel…").desired_width(f32::INFINITY).font(egui::FontId::proportional(16.0)));
                r.request_focus();
                if r.changed() {
                    self.sel = 0;
                }
                ui.add_space(6.0);
                ui.label(RichText::new(format!("{} résultat(s) · ↑ ↓ pour choisir · Entrée pour lancer · Échap pour fermer", hits.len())).size(11.0).color(FAINT));
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (k, c) in hits.iter().enumerate().take(200) {
                        let on = k == self.sel;
                        let label = format!("{}  {}", command_icon(&c.id), c.label);
                        let resp = ui.add(egui::Button::new(RichText::new(label).size(13.5).color(if on { INK_STRONG } else { INK })).fill(if on { Color32::from_white_alpha(18) } else { Color32::TRANSPARENT }).min_size(egui::vec2(ui.available_width(), 26.0)).right_text(RichText::new(&c.hint).size(11.0).color(MUTED)));
                        if on && (up || down) {
                            resp.scroll_to_me(None);
                        }
                        if resp.clicked() {
                            chosen = Some(c.id.clone());
                        }
                    }
                });
            });
        });
        if close {
            self.open = false;
        }
        let id = chosen?;
        self.open = false;
        if let Some(t) = id.strip_prefix("help:") {
            if t.is_empty() { help.toggle() } else { help.learn(t) }
            return None;
        }
        if let Some(z) = id.strip_prefix("ui-zoom:").and_then(|z| z.parse().ok()) {
            self.prefs.zoom = z;
            apply_prefs(ctx, self.prefs);
            save_prefs(self.prefs);
            return None;
        }
        if let Some(a) = id.strip_prefix("ui-accent:").and_then(|a| a.parse().ok()) {
            self.prefs.accent = a;
            apply_prefs(ctx, self.prefs);
            save_prefs(self.prefs);
            return None;
        }
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn creative_library_has_3000_unique_executable_options() {
        let options = effect_variant_cmds();
        assert_eq!(options.len(), 3_000);

        let ids: HashSet<&str> = options.iter().map(|option| option.id.as_str()).collect();
        assert_eq!(ids.len(), options.len());

        for option in options {
            let effect = effect_from_command(&option.id).expect("every generated option must decode");
            assert!((5.0..=100.0).contains(&effect.amount));
            assert!(studio_fx::stack::kind(&effect.kind).is_some());
        }
    }

    #[test]
    fn command_icons_cover_shared_command_categories() {
        assert_eq!(command_icon("fxv:Invert:50"), "✦");
        assert_eq!(command_icon("send:pixel"), "↗");
        assert_eq!(command_icon("help:Découvrir"), "?");
        assert_eq!(command_icon("ui-zoom:1.25"), "◐");
        assert_eq!(command_icon("open"), "◆");
    }

    #[test]
    fn creative_options_are_scanned_only_during_a_search() {
        assert!(!include_effect_variants(""));
        assert!(!include_effect_variants("   "));
        assert!(include_effect_variants("cinéma"));
    }
}

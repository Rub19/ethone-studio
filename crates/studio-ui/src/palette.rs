use crate::help::Help;
use crate::pro::{CHROME, FAINT, INK, INK_STRONG, LINE, MUTED};
use egui::{Align2, Color32, Context, CornerRadius, Frame, Key, KeyboardShortcut, Margin, Modifiers, RichText, Stroke};
use serde::{Deserialize, Serialize};

pub struct Cmd {
    pub label: String,
    pub hint: String,
    pub id: String,
    pub confirm: bool,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

impl Cmd {
    pub fn new(label: impl Into<String>, hint: impl Into<String>, id: impl Into<String>) -> Self {
        Cmd { label: label.into(), hint: hint.into(), id: id.into(), confirm: false, enabled: true, disabled_reason: None }
    }

    pub fn confirm(mut self) -> Self {
        self.confirm = true;
        self
    }

    pub fn confirm_if(mut self, condition: bool) -> Self {
        self.confirm = condition;
        self
    }

    pub fn enabled_if(mut self, condition: bool, reason: impl Into<String>) -> Self {
        self.enabled = condition;
        self.disabled_reason = (!condition).then(|| reason.into());
        self
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

/// A large, app-specific set of searchable options. Search keys are computed once, so looking
/// through thousands of entries stays instant; nothing is shown until a query is typed.
pub struct Library {
    pub cmds: Vec<Cmd>,
    keys: Vec<String>,
}

impl Library {
    pub fn new(cmds: Vec<Cmd>) -> Self {
        let keys = cmds.iter().map(|c| search_key(&format!("{} {}", c.label, c.hint))).collect();
        Library { cmds, keys }
    }

    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    fn matching<'a>(&'a self, words: &'a [String]) -> impl Iterator<Item = &'a Cmd> + 'a {
        self.cmds.iter().zip(&self.keys).filter(move |(_, k)| words.iter().all(|w| k.contains(w.as_str()))).map(|(c, _)| c)
    }
}

/// Named colours shared by the option libraries (French names, sRGB).
pub const NAMED_COLORS: [(&str, [u8; 3]); 48] = [
    ("Noir", [18, 18, 20]), ("Anthracite", [50, 52, 58]), ("Gris", [128, 128, 132]), ("Argent", [192, 194, 198]), ("Blanc", [250, 250, 248]), ("Ivoire", [248, 242, 222]),
    ("Crème", [244, 232, 205]), ("Beige", [222, 202, 168]), ("Sable", [214, 186, 140]), ("Caramel", [190, 128, 70]), ("Chocolat", [92, 58, 38]), ("Bordeaux", [110, 22, 40]),
    ("Rouge", [220, 38, 38]), ("Cerise", [200, 20, 70]), ("Corail", [255, 112, 92]), ("Saumon", [250, 150, 125]), ("Pêche", [255, 190, 150]), ("Orange", [255, 140, 20]),
    ("Ambre", [255, 176, 0]), ("Or", [212, 175, 55]), ("Jaune", [255, 214, 10]), ("Citron", [240, 240, 80]), ("Olive", [128, 128, 40]), ("Kaki", [150, 140, 90]),
    ("Sauge", [156, 175, 136]), ("Menthe", [150, 225, 185]), ("Vert", [48, 190, 90]), ("Émeraude", [16, 150, 110]), ("Forêt", [30, 90, 50]), ("Sarcelle", [0, 128, 128]),
    ("Turquoise", [20, 190, 200]), ("Cyan", [0, 200, 240]), ("Ciel", [120, 190, 250]), ("Azur", [10, 132, 255]), ("Bleu roi", [40, 70, 200]), ("Marine", [20, 34, 80]),
    ("Indigo", [75, 40, 160]), ("Lavande", [180, 160, 230]), ("Violet", [140, 90, 255]), ("Prune", [100, 40, 90]), ("Magenta", [220, 40, 180]), ("Fuchsia", [255, 60, 160]),
    ("Rose", [255, 140, 180]), ("Rose poudré", [230, 190, 195]), ("Lilas", [200, 170, 210]), ("Terracotta", [200, 100, 70]), ("Brique", [170, 70, 50]), ("Bronze", [160, 110, 60]),
];

pub fn color_name(c: [u8; 3]) -> &'static str {
    NAMED_COLORS.iter().min_by_key(|(_, k)| (0..3).map(|i| (k[i] as i32 - c[i] as i32).pow(2)).sum::<i32>()).map_or("", |n| n.0)
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
    } else if matches!(id, "open" | "save" | "saveas" | "timelapse" | "icon-pack") || id.starts_with("export") || id.starts_with("sprites:") {
        "▣"
    } else if matches!(id, "duplicate-layer" | "merge-visible" | "flatten-image" | "split-layer" | "fit-layer") {
        "▤"
    } else if id.starts_with("select") || matches!(id, "invert" | "deselect" | "duplicate" | "group" | "ungroup") {
        "◎"
    } else if id.starts_with("batch") || id.starts_with("blurry:") {
        "◫"
    } else if id.starts_with("normalize") || id.starts_with("fades-all:") || id.starts_with("loop:") || id.starts_with("rack:") || id.starts_with("apply:") || matches!(id, "play" | "marker" | "silences" | "silences-short" | "trim-edges" | "micro-fades" | "split-files") {
        "♫"
    } else if id.starts_with("rotate") || id.starts_with("flip") || id.starts_with("center") || id.starts_with("distribute") || id.starts_with("grid:") || id.starts_with("format:") || id.starts_with("cascade:") {
        "⌖"
    } else {
        "◆"
    }
}

fn include_effect_variants(query: &str) -> bool {
    !query.trim().is_empty()
}

fn search_key(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.to_lowercase().chars() {
        match character {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => out.push('a'),
            'ç' => out.push('c'),
            'è' | 'é' | 'ê' | 'ë' => out.push('e'),
            'ì' | 'í' | 'î' | 'ï' => out.push('i'),
            'ñ' => out.push('n'),
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => out.push('o'),
            'ù' | 'ú' | 'û' | 'ü' => out.push('u'),
            'ý' | 'ÿ' => out.push('y'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            other => out.push(other),
        }
    }
    out
}

fn query_words(query: &str) -> Vec<String> {
    search_key(query).split_whitespace().map(str::to_string).collect()
}

fn matches_words(command: &Cmd, words: &[String]) -> bool {
    let haystack = search_key(&format!("{} {} {}", command.label, command.hint, command.disabled_reason.as_deref().unwrap_or("")));
    words.iter().all(|word| haystack.contains(word))
}

fn remember_recent(recent: &mut Vec<String>, id: &str) {
    if id.is_empty() {
        return;
    }
    recent.retain(|item| item != id);
    recent.insert(0, id.to_string());
    recent.truncate(8);
}

fn rank_recent(commands: &mut Vec<&Cmd>, recent: &[String]) {
    commands.sort_by_key(|command| (!command.enabled, recent.iter().position(|id| id == &command.id).unwrap_or(usize::MAX)));
}

fn executable_id(command: &Cmd) -> Option<String> {
    command.enabled.then(|| command.id.clone())
}

fn confirmation_step(pending: &mut Option<String>, command: &Cmd) -> bool {
    if !command.enabled {
        *pending = None;
        return false;
    }
    if !command.confirm {
        *pending = None;
        return true;
    }
    if pending.as_deref() == Some(&command.id) {
        *pending = None;
        true
    } else {
        *pending = Some(command.id.clone());
        false
    }
}

pub fn validate_commands(commands: &[Cmd]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for command in commands {
        if command.label.trim().is_empty() {
            return Err(format!("La commande « {} » n'a pas de libellé", command.id));
        }
        if command.id.trim().is_empty() {
            return Err(format!("La commande « {} » n'a pas d'identifiant", command.label));
        }
        if !ids.insert(command.id.as_str()) {
            return Err(format!("Identifiant de commande dupliqué : {}", command.id));
        }
        if !command.enabled && command.disabled_reason.as_deref().is_none_or(|reason| reason.trim().is_empty()) {
            return Err(format!("La commande désactivée « {} » n'indique pas pourquoi", command.label));
        }
    }
    Ok(())
}

pub struct Palette {
    pub open: bool,
    query: String,
    sel: usize,
    prefs: Prefs,
    recent: Vec<String>,
    confirming: Option<String>,
    library: Option<fn() -> &'static Library>,
}

impl Palette {
    pub fn new(ctx: &Context) -> Self {
        let prefs = load_prefs();
        apply_prefs(ctx, prefs);
        Palette { open: false, query: String::new(), sel: 0, prefs, recent: Vec::new(), confirming: None, library: None }
    }

    /// Gives this app's option library to the palette; it is built on the first search only.
    pub fn set_library(&mut self, library: fn() -> &'static Library) {
        self.library = Some(library);
    }

    pub fn keys(&mut self, ctx: &Context) {
        let k = ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::K)) || i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::P)));
        if k {
            self.open = !self.open;
            self.query.clear();
            self.sel = 0;
            self.confirming = None;
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
        #[cfg(debug_assertions)]
        if let Err(error) = validate_commands(&app) {
            panic!("Registre Ctrl+K invalide : {error}");
        }
        let builtin = self.builtin(help);
        let words = query_words(&self.query);
        let variants = if include_effect_variants(&self.query) { effect_variant_cmds() } else { &[] };
        let library = self.library.filter(|_| !self.query.trim().is_empty()).map(|f| f());
        let mut hits: Vec<&Cmd> = app.iter().chain(&builtin).chain(variants).filter(|command| matches_words(command, &words)).chain(library.into_iter().flat_map(|l| l.matching(&words))).collect();
        if self.query.trim().is_empty() {
            rank_recent(&mut hits, &self.recent);
        }
        let visible = hits.len().min(200);
        self.sel = self.sel.min(visible.saturating_sub(1));
        let (up, down, enter, esc) = ctx.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown), i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        if down {
            self.sel = (self.sel + 1).min(visible.saturating_sub(1));
        }
        if up {
            self.sel = self.sel.saturating_sub(1);
        }
        let mut chosen = if enter { hits.get(self.sel).and_then(|command| executable_id(command)) } else { None };
        let close = esc;
        egui::Area::new(egui::Id::NULL.with("palette")).order(egui::Order::Foreground).anchor(Align2::CENTER_TOP, [0.0, 80.0]).show(ctx, |ui| {
            Frame::new().fill(CHROME).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(14)).inner_margin(Margin::same(10)).shadow(egui::epaint::Shadow { offset: [0, 16], blur: 40, spread: 0, color: Color32::from_black_alpha(170) }).show(ui, |ui| {
                ui.set_width(560.0);
                let hint = if self.library.is_some() { "Rechercher une commande, un effet, une option de la bibliothèque…" } else { "Rechercher une commande, un effet, un tutoriel…" };
                let r = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text(hint).desired_width(f32::INFINITY).font(egui::FontId::proportional(16.0)));
                r.request_focus();
                if r.changed() {
                    self.sel = 0;
                    self.confirming = None;
                }
                ui.add_space(6.0);
                let recent = if self.query.trim().is_empty() && !self.recent.is_empty() { " · récentes en premier" } else { "" };
                ui.label(RichText::new(format!("{} résultat(s){recent} · ↑ ↓ pour choisir · Entrée pour lancer · Échap pour fermer", hits.len())).size(11.0).color(FAINT));
                let confirmation_label = self.confirming.as_ref().and_then(|id| hits.iter().find(|command| &command.id == id)).map(|command| command.label.as_str());
                if let Some(label) = &confirmation_label {
                    ui.label(RichText::new(format!("⚠ Confirmer « {label} » avec Entrée ou un second clic")).size(12.0).color(Color32::from_rgb(255, 180, 60)));
                }
                if let Some(reason) = hits.get(self.sel).and_then(|command| (!command.enabled).then_some(command.disabled_reason.as_deref()).flatten()) {
                    ui.label(RichText::new(format!("Indisponible : {reason}")).size(12.0).color(MUTED));
                }
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (k, c) in hits.iter().enumerate().take(200) {
                        let on = k == self.sel;
                        let label = format!("{}  {}", if !c.enabled { "○" } else if c.confirm { "⚠" } else { command_icon(&c.id) }, c.label);
                        let detail = c.disabled_reason.as_deref().map_or(c.hint.as_str(), |reason| reason);
                        let button = egui::Button::new(RichText::new(label).size(13.5).color(if !c.enabled { MUTED } else if on { INK_STRONG } else { INK })).fill(if on && c.enabled { Color32::from_white_alpha(18) } else { Color32::TRANSPARENT }).min_size(egui::vec2(ui.available_width(), 26.0)).right_text(RichText::new(detail).size(11.0).color(MUTED));
                        let resp = ui.add_enabled(c.enabled, button);
                        if on && (up || down) {
                            resp.scroll_to_me(None);
                        }
                        if resp.clicked() {
                            self.sel = k;
                            chosen = executable_id(c);
                        }
                    }
                });
            });
        });
        if close {
            self.open = false;
            self.confirming = None;
            return None;
        }
        let id = chosen?;
        let Some(command) = app.iter().chain(&builtin).chain(variants).find(|command| command.id == id) else { return None };
        if !confirmation_step(&mut self.confirming, command) {
            return None;
        }
        self.open = false;
        remember_recent(&mut self.recent, &id);
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
        assert_eq!(command_icon("export-page"), "▣");
        assert_eq!(command_icon("select-inverse"), "◎");
        assert_eq!(command_icon("batch-sync"), "◫");
        assert_eq!(command_icon("normalize-lufs"), "♫");
        assert_eq!(command_icon("rotate-right"), "⌖");
        assert_eq!(command_icon("flatten-image"), "▤");
        assert_eq!(command_icon("open"), "▣");
        assert_eq!(command_icon("remove-bg"), "◆");
    }

    #[test]
    fn creative_options_are_scanned_only_during_a_search() {
        assert!(!include_effect_variants(""));
        assert!(!include_effect_variants("   "));
        assert!(include_effect_variants("cinéma"));
    }

    #[test]
    fn recent_commands_are_unique_newest_first_and_bounded() {
        let mut recent = Vec::new();
        for id in 0..10 {
            remember_recent(&mut recent, &format!("cmd-{id}"));
        }
        remember_recent(&mut recent, "cmd-4");
        assert_eq!(recent.first().map(String::as_str), Some("cmd-4"));
        assert_eq!(recent.len(), 8);
        assert_eq!(recent.iter().collect::<HashSet<_>>().len(), recent.len());
    }

    #[test]
    fn command_search_ignores_case_and_accents() {
        let command = Cmd::new("Synchroniser les réglages", "Photos choisies", "sync");
        assert!(matches_words(&command, &query_words("REGLAGES choisies")));
        assert!(!matches_words(&command, &query_words("audio")));
    }

    #[test]
    fn recent_commands_are_ranked_before_the_default_order() {
        let commands = [Cmd::new("Ouvrir", "", "open"), Cmd::new("Exporter", "", "export"), Cmd::new("Aplatir", "", "flatten")];
        let mut hits: Vec<&Cmd> = commands.iter().collect();
        rank_recent(&mut hits, &["flatten".into(), "open".into()]);
        assert_eq!(hits.iter().map(|command| command.id.as_str()).collect::<Vec<_>>(), ["flatten", "open", "export"]);
    }

    #[test]
    fn command_registry_rejects_empty_and_duplicate_entries() {
        assert!(validate_commands(&[Cmd::new("Ouvrir", "", "open"), Cmd::new("Exporter", "", "export")]).is_ok());
        assert!(validate_commands(&[Cmd::new("", "", "open")]).is_err());
        assert!(validate_commands(&[Cmd::new("Ouvrir", "", "")]).is_err());
        assert!(validate_commands(&[Cmd::new("A", "", "same"), Cmd::new("B", "", "same")]).is_err());
    }

    #[test]
    fn sensitive_command_requires_two_matching_confirmations() {
        let danger = Cmd::new("Aplatir", "", "flatten").confirm();
        let other = Cmd::new("Réinitialiser", "", "reset").confirm();
        let mut pending = None;

        assert!(!confirmation_step(&mut pending, &danger));
        assert_eq!(pending.as_deref(), Some("flatten"));
        assert!(!confirmation_step(&mut pending, &other));
        assert_eq!(pending.as_deref(), Some("reset"));
        assert!(confirmation_step(&mut pending, &other));
        assert!(pending.is_none());
    }

    #[test]
    fn regular_command_executes_immediately_and_cancels_pending_confirmation() {
        let command = Cmd::new("Ouvrir", "", "open");
        let mut pending = Some("flatten".to_string());
        assert!(confirmation_step(&mut pending, &command));
        assert!(pending.is_none());
    }

    #[test]
    fn library_search_uses_precomputed_keys() {
        let lib = Library::new(vec![Cmd::new("Look Portra · 40 %", "Pellicule", "lm:a"), Cmd::new("Palette Corail", "Triade", "vc:b")]);
        let words = query_words("portra");
        assert_eq!(lib.matching(&words).map(|c| c.id.as_str()).collect::<Vec<_>>(), ["lm:a"]);
        assert_eq!(lib.matching(&query_words("CORAIL triade")).count(), 1);
        assert_eq!(color_name([250, 110, 90]), "Corail");
    }

    #[test]
    fn disabled_command_exposes_reason_and_cannot_execute() {
        let disabled = Cmd::new("Dupliquer", "", "duplicate").enabled_if(false, "Sélectionne d'abord un objet");
        let enabled = Cmd::new("Ouvrir", "", "open").enabled_if(true, "sans document");
        assert!(!disabled.enabled);
        assert_eq!(disabled.disabled_reason.as_deref(), Some("Sélectionne d'abord un objet"));
        assert!(executable_id(&disabled).is_none());
        assert!(enabled.enabled);
        assert!(enabled.disabled_reason.is_none());
        assert_eq!(executable_id(&enabled).as_deref(), Some("open"));
    }

    #[test]
    fn disabled_commands_are_ranked_after_available_commands() {
        let commands = [
            Cmd::new("Indisponible récente", "", "recent-disabled").enabled_if(false, "Aucune sélection"),
            Cmd::new("Disponible", "", "available"),
            Cmd::new("Disponible récente", "", "recent-available"),
        ];
        let mut hits: Vec<&Cmd> = commands.iter().collect();
        rank_recent(&mut hits, &["recent-disabled".into(), "recent-available".into()]);
        assert_eq!(hits.iter().map(|command| command.id.as_str()).collect::<Vec<_>>(), ["recent-available", "available", "recent-disabled"]);
    }
}

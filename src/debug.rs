use std::collections::HashMap;
use std::fmt::Debug;

pub fn good_debug<T: GoodDebug>(t: &T) -> String {
    let mut f = Fmtter::new();
    t.good_debug(&mut f);
    f.result
}

pub struct Fmtter {
    pub indent: u8,
    pub result: String,
}
impl Fmtter {
    pub fn new() -> Self {
        Self {
            indent: 0,
            result: String::new(),
        }
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }
    pub fn unindent(&mut self) {
        self.indent -= 1;
    }

    pub fn push_str(&mut self, s: &str) {
        self.result.push_str(s);
    }
    pub fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.result.push_str("    ");
        }
    }

    pub fn push_field<T: GoodDebug>(&mut self, name: &str, t: &T) {
        self.push_indent();
        self.push_str(name);
        self.result.push_str(": ");
        t.good_debug(self);
        self.result.push('\n');
    }
    pub fn push_last_field<T: GoodDebug>(&mut self, name: &str, t: &T) {
        self.push_indent();
        self.push_str(name);
        self.result.push_str(": ");
        t.good_debug(self);
    }

    pub fn print(self) {
        use std::io::Write;
        std::io::stdout().write(self.result.as_bytes()).unwrap();
    }
}
pub trait GoodDebug {
    fn good_debug(&self, f: &mut Fmtter);
}

// *** LIST IMPLS ***
impl<T: GoodDebug> GoodDebug for [T] {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("list");
        if self.is_empty() {
            return;
        }
        f.push_str("\n");
        f.indent();
        for e in self {
            f.push_indent();
            e.good_debug(f);
            f.push_str("\n");
        }
        f.unindent();
    }
}
impl<T: GoodDebug> GoodDebug for Vec<T> {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("list");
        if self.is_empty() {
            return;
        }
        f.push_str("\n");
        f.indent();
        for e in self {
            f.push_indent();
            e.good_debug(f);
            f.push_str("\n");
        }
        f.unindent();
    }
}
impl<K: Debug, V: GoodDebug> GoodDebug for HashMap<K, V> {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("map");
        if self.is_empty() {
            return;
        }
        f.push_str("\n");
        f.indent();
        for (key, e) in self {
            f.push_indent();
            f.push_str(&format!("{:?} : ", key));
            e.good_debug(f);
            f.push_str("\n");
        }
        f.unindent();
    }
}

// *** FROM DEBUG DEBUG IMPLS ***
macro_rules! debug_as_good_debug {
	 ($ty:ty) => {
        impl GoodDebug for $ty {
            fn good_debug(&self, f: &mut Fmtter) {
                f.push_str(&format!("{:?}", self));
            }
        }
    };
	($($ty:ty),*$(,)?) => {
		$(debug_as_good_debug!($ty);)*
	};
}
debug_as_good_debug!(
    !,
    String,
    bool,
    [f32; 3],
    u8,
    (),
    usize,
    crate::BitField,
    crate::SimId,
    eframe::egui::Pos2,
    eframe::egui::Color32,
);

// *** CUSTOM IMPLS ***
impl<T: Debug> GoodDebug for crate::LinkTarget<T> {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str(&format!("{:?}", self));
    }
}
impl<T: Debug> GoodDebug for crate::LinkStart<T> {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str(&format!("{:?}", self));
    }
}

impl<S: GoodDebug, C: GoodDebug, G: GoodDebug> GoodDebug for crate::DeviceData<S, C, G> {
    fn good_debug(&self, f: &mut Fmtter) {
        match self {
            Self::CombGate(e) => {
                f.push_str("DeviceData::CombGate / ");
                e.good_debug(f);
            }
            Self::Chip(e) => {
                f.push_str("DeviceData::Chip / ");
                e.good_debug(f);
            }
            Self::Light(e) => {
                f.push_str("DeviceData::Light / ");
                e.good_debug(f);
            }
            Self::Switch(e) => {
                f.push_str("DeviceData::Switch / ");
                e.good_debug(f);
            }
        }
    }
}

impl GoodDebug for crate::TruthTable {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("TruthTable\n");
        f.indent();
        f.push_field("num_inputs", &self.num_inputs);
        f.push_field("num_outputs", &self.num_outputs);
        f.push_field("map", &self.map);
        f.unindent();
    }
}

impl GoodDebug for crate::scene::Scene {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Scene\n");
        f.indent();
        f.push_field("name", &self.name);
        f.push_field("color", &self.color);
        f.push_field("combinational", &self.combinational);
        f.push_field("inputs", &self.inputs);
        f.push_field("outputs", &self.outputs);
        f.push_field("devices", &self.devices);
        f.push_last_field("writes", &self.writes);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::Input {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Input\n");
        f.indent();
        f.push_field("label", &self.label);
        f.push_field("state", &self.state);
        f.push_last_field("links", &self.links);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::Output {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Output\n");
        f.indent();
        f.push_field("label", &self.label);
        f.push_last_field("state", &self.state);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::Device {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Device\n");
        f.indent();
        f.push_field("preset", &self.preset);
        f.push_field("pos", &self.pos);
        f.push_field("data", &self.data);
        f.push_last_field("links", &self.links);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::Write {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Write\n");
        f.indent();
        f.push_field("target", &self.target);
        f.push_last_field("state", &self.state);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::CombGate {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("CombGate\n");
        f.indent();
        f.push_field("preset", &self.preset);
        f.push_field("input", &self.input);
        f.push_field("output", &self.output);
        f.push_last_field("table", &self.table);
        f.unindent();
    }
}

impl GoodDebug for crate::scene::chip::Chip {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Chip\n");
        f.indent();
        f.push_field("writes", &self.writes);
        f.push_field("inputs", &self.inputs);
        f.push_field("outputs", &self.outputs);
        f.push_last_field("devices", &self.devices);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::chip::Input {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Input\n");
        f.indent();
        f.push_field("state", &self.state);
        f.push_last_field("links", &self.links);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::chip::Output {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Output / ");
        self.state.good_debug(f);
    }
}
impl GoodDebug for crate::scene::chip::Device {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Device\n");
        f.indent();
        f.push_field("preset", &self.preset);
        f.push_field("links", &self.links);
        f.push_last_field("data", &self.data);
        f.unindent();
    }
}
impl GoodDebug for crate::scene::chip::Write {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Write\n");
        f.indent();
        f.push_field("delay", &self.delay);
        f.push_field("target", &self.target);
        f.push_field("state", &self.state);
        f.unindent();
    }
}

impl GoodDebug for crate::preset::chip::UnnestedChip {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("UnnestedChip\n");
        f.indent();
        f.push_field("id", &self.id);
        f.push_field("chip", &self.chip);
        f.push_field("output_links", &self.output_links);
        f.push_last_field("device_ids", &self.device_ids);
        f.unindent();
    }
}
impl<'a> GoodDebug for crate::preset::chip::Unnester<'a> {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Unnester\n");
        f.indent();

        f.push_field("scene", self.scene);
        f.push_last_field("chips", &self.chips);

        f.unindent();
    }
}
impl GoodDebug for crate::preset::chip::Chip {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Chip\n");
        f.indent();

        f.push_field("name", &self.name);
        f.push_field("color", &self.color);
        f.push_field("inputs", &self.inputs);
        f.push_field("outputs", &self.outputs);
        f.push_last_field("devices", &self.devices);

        f.unindent();
    }
}
impl GoodDebug for crate::preset::chip::Input {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Input\n");
        f.indent();

        f.push_field("label", &self.label);
        f.push_last_field("links", &self.links);

        f.unindent();
    }
}
impl GoodDebug for crate::preset::chip::Output {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Output / ");
        self.label.good_debug(f);
    }
}
impl GoodDebug for crate::preset::chip::Device {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Device\n");
        f.indent();

        f.push_field("preset", &self.preset);
        f.push_field("data", &self.data);
        f.push_last_field("links", &self.links);

        f.unindent();
    }
}

impl GoodDebug for crate::preset::IoLabel {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str(&format!("{:?}", self));
    }
}
impl GoodDebug for crate::preset::CombGate {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("CombGate\n");
        f.indent();

        f.push_field("name", &self.name);
        f.push_field("color", &self.color);
        f.push_field("inputs", &self.inputs);
        f.push_field("outputs", &self.outputs);
        f.push_last_field("table", &self.table);

        f.unindent();
    }
}

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
impl<A: GoodDebug, B: GoodDebug> GoodDebug for (A, B) {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("tuple");
        f.push_str("\n");
        f.indent();

        f.push_indent();
        self.0.good_debug(f);
        f.push_str("\n");

        f.push_indent();
        self.1.good_debug(f);
        f.push_str("\n");

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
macro_rules! impl_from_debug {
	 ($ty:ty) => {
        impl GoodDebug for $ty {
            fn good_debug(&self, f: &mut Fmtter) {
                f.push_str(&format!("{:?}", self));
            }
        }
    };
	($($ty:ty),*$(,)?) => {
		$(impl_from_debug!($ty);)*
	};
}
impl_from_debug!(
    [f32; 2],
    [f32; 3],
    [f32; 4],
    bool,
    u8,
    u32,
    f32,
    usize,
    String,
    crate::IntId,
    eframe::egui::Pos2,
    eframe::egui::Vec2,
    eframe::egui::Rect,
    eframe::egui::Color32,
);

impl<T: Debug> GoodDebug for crate::DeviceInput<T> {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str(&format!("{:?}", self));
    }
}

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

macro_rules! impl_struct {
    ($($path:tt),*:$name:ident{$($field:ident),*}) => {
    	impl GoodDebug for $($path::)*$name {
    		fn good_debug(&self, f: &mut Fmtter) {
    			f.push_str(concat!(stringify!($name), "\n"));
    			f.indent();
    			$(
    				f.push_field(stringify!($field), &self.$field);
    			)*
    			f.unindent();
    		}
    	}
    };
    ($($path:tt),*:$name:ident<$($t:ident),*> {$($field:ident),*}) => {
    	impl<$($t: GoodDebug + Debug),*> GoodDebug for $($path::)*$name<$($t),*> {
    		fn good_debug(&self, f: &mut Fmtter) {
    			f.push_str(concat!(stringify!($name), "\n"));
    			f.indent();
    			$(
    				f.push_field(stringify!($field), &self.$field);
    			)*
    			f.unindent();
    		}
    	}
    };
}
macro_rules! impl_enum_1 {
    ($($path:tt),*:$name:ident {$($var:ident),*}) => {
        impl GoodDebug for $($path::)*$name {
            fn good_debug(&self, f: &mut Fmtter) {
                f.push_str(concat!(stringify!($name), "::"));
                match self {
                	$(Self::$var(e) => e.good_debug(f),)*
                }
            }
        }
    };
}

impl_struct!(crate:BitField { len, data });
impl_struct!(crate:TruthTable { num_inputs, num_outputs, map });
impl_struct!(crate:DeviceVisuals { name, color });

impl_struct!(crate,preset:Io { name, implicit });
impl_struct!(crate,preset:CombGate { inputs, outputs, table });
impl_enum_1!(crate,preset:PresetData { CombGate, Chip });
impl_struct!(crate,preset:Preset { vis, data });
impl_struct!(crate,preset:Cat { name, presets, next_preset_id });
impl_struct!(crate,preset:Presets { cats, next_cat_id });

impl_struct!(crate,preset,chip:Chip { inputs, outputs, comb_gates });
impl_struct!(crate,preset,chip:CombGate { table, links });
impl_struct!(crate,preset,chip:Input { preset, links });

impl_struct!(crate,scene:Write { target, state });
impl_enum_1!(crate,scene:DeviceData { CombGate, Chip });
impl_struct!(crate,scene:Device { pos, size, data, links, vis, input_presets, output_presets });
impl_struct!(crate,scene:CombGate { input, output, table });
impl_struct!(crate,scene:Input { preset, state, links, y_pos });
impl_struct!(crate,scene:Output { preset, state });
impl_struct!(crate,scene:Scene { rect, inputs, outputs, devices, writes });

impl_struct!(crate,scene,chip:Device { links, data });
impl_struct!(crate,scene,chip:Input { state, links });
impl_struct!(crate,scene,chip:Output { state });
impl_struct!(crate,scene,chip:Chip { writes, inputs, outputs, devices });
impl_struct!(crate,scene,chip:Write { delay, target, state });

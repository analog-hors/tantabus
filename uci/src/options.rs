use tantabus::search::{EngineOptions, SearchParams};

pub struct UciOptions {
    pub engine_options: EngineOptions,
    pub search_params: SearchParams,
    pub cache_table_size: usize,
    pub chess960: bool
}

pub enum UciOptionKind {
    Check {
        default: bool
    },
    Spin {
        default: i64,
        min: i64,
        max: i64
    }
}

type OptionHandler = Box<dyn Fn(&mut UciOptions, &str)>;

type UciOption = (String, UciOptionKind, OptionHandler);

pub struct UciOptionsHandler {
    pub handlers: Vec<UciOption>,
    pub options: UciOptions
}

const MEGABYTE: usize = 1_000_000;

impl UciOptionsHandler {
    pub fn new() -> Self {
        use UciOptionKind::*;

        let options = UciOptions {
            engine_options: EngineOptions::default(),
            search_params: SearchParams::default(),
            cache_table_size: 16 * MEGABYTE,
            chess960: false
        };
        let handlers = vec![
            make_option("UCI_Chess960", Check {
                default: options.chess960
            }, |o, v| {
                o.chess960 = v.parse().unwrap();
            }),
            make_option("Hash", Spin {
                default: (options.cache_table_size / MEGABYTE) as i64,
                min: 0,
                max: 64_000 // 64 Gigabytes
            }, |o, v| {
                o.cache_table_size = v.parse::<usize>().unwrap() * MEGABYTE;
            }),
            make_option("Threads", Spin {
                default: 1,
                min: 1,
                max: 4096
            }, |o, v| {
                o.engine_options.threads = v.parse().unwrap();
            })
        ];
        macro_rules! add_search_param_handlers {
            ($([$($field:tt)*])*) => {$({
                let name = concat!("TUNE_", stringify!($($field)*)).replace(' ', "");
                let option = Spin {
                    name: ,
                    default: Some(Tunable::to_tune_value(options.search_params.$($field)*)),
                    min: i32::MIN as i64,
                    max: i32::MAX as i64
                };
                let handler = |o, v| {
                    o.search_params.$($field)* = Tunable::from_tune_value(v.parse().unwrap());
                };
                handlers.push(make_option(name, option, handler));
            })*}
        }
        // Modify for exposing search params for tuning
        add_search_param_handlers! {
            // [lmr.min_depth]
            // [lmr.base_reduction]
            // [lmr.div]
            // [lmr.history_reduction_div]
            // [nmp.base_reduction]
            // [nmp.margin_div]
            // [nmp.margin_max_reduction]
            // [lmp.quiets_to_check[0]]
            // [lmp.quiets_to_check[1]]
            // [lmp.quiets_to_check[2]]
            // [fp.margins[0]]
            // [fp.margins[1]]
            // [rfp.base_margin]
            // [rfp.max_depth]
        }

        Self {
            handlers,
            options
        }
    }

    pub fn update(&mut self, name: &str, value: &str) {
        for (option_name, _, handler) in &self.handlers {
            if option_name == name {
                handler(&mut self.options, value);
            }
        }
    }
}

fn make_option(name: &str, option: UciOptionKind, handler: impl Fn(&mut UciOptions, &str) + 'static) -> UciOption {
    (name.to_owned(), option, Box::new(handler))
}

trait Tunable {
    fn to_tune_value(self) -> i64;
    fn from_tune_value(value: i64) -> Self;
}

macro_rules! impl_tunable {
    ($($type:ty),*) => {$(
        impl Tunable for $type {
            fn to_tune_value(self) -> i64 {
                self as i64
            }

            fn from_tune_value(value: i64) -> Self {
                value as _
            }
        }
    )*}
}
impl_tunable!(i8, u8, i16, u16, i32, u32);

impl Tunable for f32 {
    fn to_tune_value(self) -> i64 {
        (self * 1000.0).round() as i64
    }

    fn from_tune_value(value: i64) -> Self {
        value as f32 / 1000.0
    }
}

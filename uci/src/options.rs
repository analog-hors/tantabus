use indexmap::IndexMap;
use tantabus::search::EngineOptions;
use vampirc_uci::UciOptionConfig;

pub struct UciOptions {
    pub engine_options: EngineOptions,
    pub cache_table_size: usize,
    pub chess960: bool
}

type Handler = Box<dyn Fn(&mut UciOptions, String)>;

pub struct UciOptionsHandler {
    pub handlers: IndexMap<String, (UciOptionConfig, Handler)>,
    pub options: UciOptions
}

const MEGABYTE: usize = 1_000_000;

impl UciOptionsHandler {
    pub fn new() -> Self {
        let options = UciOptions {
            engine_options: EngineOptions::default(),
            cache_table_size: 16 * MEGABYTE,
            chess960: false
        };
        let mut handlers = IndexMap::new();
        macro_rules! add_handlers {
            ($($option:expr => $handler:expr)*) => {
                $({
                    let option = $option;
                    let name = match &option {
                        UciOptionConfig::Check { name, .. } => name,
                        UciOptionConfig::Spin { name, .. } => name,
                        UciOptionConfig::Combo { name, .. } => name,
                        UciOptionConfig::Button { name } => name,
                        UciOptionConfig::String { name, .. } => name
                    }.to_owned();
                    let handler: Handler = Box::new($handler);
                    handlers.insert(name, (option, handler));
                })*
            }
        }
        add_handlers! {
            UciOptionConfig::Check {
                name: "UCI_Chess960".to_owned(),
                default: Some(options.chess960)
            } => |options, value| {
                options.chess960 = value
                    .parse()
                    .unwrap();
            }
            UciOptionConfig::Spin {
                name: "Hash".to_owned(),
                default: Some((options.cache_table_size / MEGABYTE) as i64),
                min: Some(0),
                max: Some(64_000) //64 Gigabytes
            } => |options, value| {
                options.cache_table_size = value
                    .parse::<usize>()
                    .unwrap()
                    * MEGABYTE
            }
            UciOptionConfig::Spin {
                name: "Threads".to_owned(),
                default: Some(1),
                min: Some(1),
                max: Some(1)
            } => |_, _| {
                // Implementation of the "Laziest SMP" algorithm
            }
        }
        Self {
            handlers,
            options
        }
    }

    pub fn update(&mut self, key: &str, value: Option<String>) {
        if let Some((_, handler)) = self.handlers.get(key) {
            handler(&mut self.options, value.unwrap())
        }
    }
}

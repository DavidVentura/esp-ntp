use crate::clock;
use chrono::DateTime;
use chrono_tz::{Tz, TZ_VARIANTS};
use esp_idf_svc::nvs::EspDefaultNvs;

pub struct ClockFace {
    tz: Tz,
    nvs: Option<EspDefaultNvs>,
}

impl ClockFace {
    const TZ_KEY: &'static str = "clock_tz";

    pub fn new(tz_name: &str) -> ClockFace {
        let tz: Tz = tz_name.parse().unwrap();
        ClockFace { tz, nvs: None }
    }

    pub fn with_nvs(nvs: EspDefaultNvs) -> ClockFace {
        let mut buf: &mut [u8] = &mut [0; 32];
        let tz_name = nvs.get_str(Self::TZ_KEY, &mut buf);
        println!("tz_name is {:?}", tz_name);

        let tz: Tz = match tz_name {
            Ok(opt) => match opt {
                Some(s) => {
                    println!("Found valid string {}", s);
                    match s.parse() {
                        Ok(t) => t,
                        Err(e) => {
                            println!("Could not parse TZ: {}", e);
                            Tz::UTC
                        }
                    }
                }
                None => {
                    println!("Did not find str");
                    Tz::UTC
                }
            },
            Err(e) => {
                println!("Could read string from nvs: {}", e);
                Tz::UTC
            }
        };

        ClockFace { tz, nvs: Some(nvs) }
    }

    pub fn current_tz(&self) -> Tz {
        self.tz
    }

    pub fn set_tz(&mut self, tz_name: &str) {
        self.tz = tz_name.parse().unwrap();
        match &mut self.nvs {
            Some(n) => {
                let res = n.set_str(Self::TZ_KEY, tz_name);
                println!("Storing {tz_name} res = {:?}", res);
            }
            None => (),
        }
    }
    pub fn avail_tz(&self) -> &'static [Tz] {
        &TZ_VARIANTS
    }

    pub fn now(&self) -> DateTime<Tz> {
        clock::now().with_timezone(&self.tz)
    }
}

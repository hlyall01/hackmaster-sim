use std::env;

#[derive(Default)]
pub struct AutobattlerArgs {
    pub auto_screenshots: bool,
    pub auto_screenshot_interval: Option<f32>,
    pub auto_screenshot_count: Option<u32>,
    pub auto_start_run: bool,
    pub headless_screenshots: bool,
    pub sprite_review: bool,
}

impl AutobattlerArgs {
    pub fn parse() -> Self {
        let mut args = Self::default();
        let mut iter = env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--auto-screenshots" => args.auto_screenshots = true,
                "--auto-screenshot-interval" => {
                    if let Some(value) = iter.next() {
                        if let Ok(parsed) = value.parse::<f32>() {
                            args.auto_screenshot_interval = Some(parsed);
                        }
                    }
                }
                "--auto-screenshot-count" => {
                    if let Some(value) = iter.next() {
                        if let Ok(parsed) = value.parse::<u32>() {
                            args.auto_screenshot_count = Some(parsed);
                        }
                    }
                }
                "--auto-start-run" => args.auto_start_run = true,
                "--headless-screenshots" | "--headless" => args.headless_screenshots = true,
                "--sprite-review" => args.sprite_review = true,
                _ => {}
            }
        }
        args
    }
}

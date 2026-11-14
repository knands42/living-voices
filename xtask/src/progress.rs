use living_voices::provision::human_bytes;

const STEP: u64 = 8 * 1024 * 1024;

#[derive(Default)]
pub struct Progress {
    printed: u64,
}

impl Progress {
    pub fn report(&mut self, file: &str, written: u64, total: Option<u64>) {
        let complete = total.is_some_and(|total| written >= total);
        if !complete && written.saturating_sub(self.printed) < STEP {
            return;
        }

        self.printed = written;
        let line = match total {
            Some(total) => format!("{file}: {} / {}", human_bytes(written), human_bytes(total)),
            None => format!("{file}: {}", human_bytes(written)),
        };

        print!("\r{line}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    pub fn finish(&self) {
        println!();
    }
}

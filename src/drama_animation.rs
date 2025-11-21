use colored::*;
use std::thread;
use std::time::Duration;

/// Catalog of funny messages for different drama scenarios
pub struct DramaMessages;

impl DramaMessages {
    pub fn lockfile_detected() -> &'static [&'static str] {
        &[
            "🔍 Sniffing around for lockfiles...",
            "🕵️  Detective mode: ON",
            "📦 Scanning for package manager artifacts...",
            "🔎 Looking for evidence of PM crimes...",
            "🎯 Lockfile radar activated...",
        ]
    }

    pub fn multiple_lockfiles(count: usize) -> &'static str {
        match count {
            2 => "😬 Uh oh, found a couple of lockfiles playing together...",
            3 => "😰 Three's a crowd! Multiple lockfiles detected!",
            4 => "🤯 FOUR lockfiles?! Someone's been busy!",
            _ => "💀 This is getting out of hand! Too many lockfiles!",
        }
    }

    pub fn dockerfile_check() -> &'static [&'static str] {
        &[
            "🐳 Peeking into the Dockerfile...",
            "🔍 Checking Docker's preferences...",
            "🐋 What does Docker have to say?",
            "📋 Reading Docker's diary...",
        ]
    }

    pub fn dockerfile_match() -> &'static str {
        "✅ Docker agrees with your lockfiles! +0 drama"
    }

    pub fn dockerfile_conflict(pm: &str) -> String {
        format!("⚠️  Docker wants {}! +20 drama points", pm.cyan().bold())
    }

    pub fn ci_check() -> &'static [&'static str] {
        &[
            "🔧 Investigating CI/CD configuration...",
            "🤖 What does the CI bot prefer?",
            "📊 Checking pipeline preferences...",
            "🔍 Reading CI's mind...",
        ]
    }

    pub fn ci_match() -> &'static str {
        "✅ CI is happy with your setup! +0 drama"
    }

    pub fn ci_conflict(pm: &str) -> String {
        format!("⚠️  CI demands {}! +20 drama points", pm.cyan().bold())
    }

    pub fn infrastructure_war(docker_pm: &str, ci_pm: &str) -> String {
        format!(
            "💥 INFRASTRUCTURE WAR! Docker wants {} but CI wants {}! +10 drama",
            docker_pm.cyan().bold(),
            ci_pm.magenta().bold()
        )
    }

    pub fn calculating() -> &'static [&'static str] {
        &[
            "🧮 Crunching the numbers...",
            "📊 Running drama calculations...",
            "🎲 Rolling the chaos dice...",
            "⚡ Computing drama coefficient...",
            "🔮 Consulting the chaos oracle...",
        ]
    }

    pub fn final_score(score: u8) -> String {
        match score {
            0..=20 => format!(
                "🟢 {} - Everything is zen! 🧘",
                format!("{}%", score).bright_green().bold()
            ),
            21..=40 => format!(
                "🟡 {} - Just a little spicy 🌶️",
                format!("{}%", score).bright_yellow().bold()
            ),
            41..=60 => format!(
                "🟠 {} - Things are heating up! 🔥",
                format!("{}%", score).bright_yellow().bold()
            ),
            61..=80 => format!(
                "🔴 {} - DRAMA ALERT! 🚨",
                format!("{}%", score).bright_red().bold()
            ),
            81..=100 => format!(
                "💥 {} - ABSOLUTE CHAOS! 🌪️",
                format!("{}%", score).bright_red().bold()
            ),
            _ => format!("❓ {}% - Unknown territory", score),
        }
    }

    pub fn score_commentary(score: u8) -> &'static str {
        match score {
            0..=20 => "Your project is a peaceful garden of package management harmony.",
            21..=40 => "A few bumps in the road, but nothing we can't handle!",
            41..=60 => "Houston, we have a problem... but it's fixable!",
            61..=80 => "This is fine. Everything is fine. (It's not fine.)",
            81..=100 => "🔥 This is fine. 🔥 (Narrator: It was not fine.)",
            _ => "We've entered uncharted chaos territory.",
        }
    }
}

/// Animated drama score calculator
pub struct DramaAnimator {
    delay_ms: u64,
}

impl Default for DramaAnimator {
    fn default() -> Self {
        Self { delay_ms: 150 }
    }
}

impl DramaAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    fn print_with_delay(&self, message: &str) {
        println!("{}", message);
        thread::sleep(Duration::from_millis(self.delay_ms));
    }

    fn random_message<'a>(messages: &'a [&'a str]) -> &'a str {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = RandomState::new().build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        let index = (hasher.finish() as usize) % messages.len();
        messages[index]
    }

    pub fn animate_detection(
        &self,
        lockfiles: &[(String, String)],
        docker_pm: &Option<String>,
        ci_pm: &Option<String>,
    ) -> u8 {
        println!(
            "\n{}",
            "🎬 Starting Package Manager Drama Analysis..."
                .bright_cyan()
                .bold()
        );
        thread::sleep(Duration::from_millis(self.delay_ms * 2));

        // Phase 1: Lockfile detection
        self.print_with_delay(&format!(
            "\n{}",
            Self::random_message(DramaMessages::lockfile_detected()).dimmed()
        ));

        let mut score = 0u8;

        if lockfiles.is_empty() {
            self.print_with_delay(&format!(
                "   {}",
                "No lockfiles found. Starting fresh!".green()
            ));
        } else if lockfiles.len() == 1 {
            let (file, pm) = &lockfiles[0];
            self.print_with_delay(&format!(
                "   {} Found {} ({})",
                "✓".green(),
                file.bright_white(),
                pm.cyan()
            ));
        } else {
            // Multiple lockfiles - drama begins!
            self.print_with_delay(&format!(
                "   {}",
                DramaMessages::multiple_lockfiles(lockfiles.len()).yellow()
            ));

            for (file, pm) in lockfiles {
                self.print_with_delay(&format!(
                    "   {} {} ({})",
                    "•".yellow(),
                    file.bright_white(),
                    pm.cyan()
                ));
            }

            score += 40;
            if lockfiles.len() > 2 {
                let extra = ((lockfiles.len() - 2) * 10).min(20) as u8;
                score += extra;
                self.print_with_delay(&format!(
                    "   {} +{} drama points for extra lockfiles",
                    "📈".yellow(),
                    40 + extra
                ));
            } else {
                self.print_with_delay(&format!("   {} +40 drama points", "📈".yellow()));
            }
        }

        // Phase 2: Docker check
        if docker_pm.is_some() || lockfiles.len() > 1 {
            thread::sleep(Duration::from_millis(self.delay_ms));
            self.print_with_delay(&format!(
                "\n{}",
                Self::random_message(DramaMessages::dockerfile_check()).dimmed()
            ));
        }

        if let Some(docker_pm_name) = docker_pm {
            let lockfile_pms: Vec<&str> = lockfiles.iter().map(|(_, pm)| pm.as_str()).collect();

            self.print_with_delay(&format!(
                "   {} Dockerfile uses {}",
                "🐳",
                docker_pm_name.cyan().bold()
            ));

            if !lockfile_pms.contains(&docker_pm_name.as_str()) {
                score += 20;
                self.print_with_delay(&format!(
                    "   {}",
                    DramaMessages::dockerfile_conflict(docker_pm_name)
                ));
            } else {
                self.print_with_delay(&format!("   {}", DramaMessages::dockerfile_match().green()));
            }
        }

        // Phase 3: CI/CD check
        if ci_pm.is_some() || lockfiles.len() > 1 {
            thread::sleep(Duration::from_millis(self.delay_ms));
            self.print_with_delay(&format!(
                "\n{}",
                Self::random_message(DramaMessages::ci_check()).dimmed()
            ));
        }

        if let Some(ci_pm_name) = ci_pm {
            let lockfile_pms: Vec<&str> = lockfiles.iter().map(|(_, pm)| pm.as_str()).collect();

            self.print_with_delay(&format!(
                "   {} CI/CD uses {}",
                "🤖",
                ci_pm_name.magenta().bold()
            ));

            if !lockfile_pms.contains(&ci_pm_name.as_str()) {
                score += 20;
                self.print_with_delay(&format!("   {}", DramaMessages::ci_conflict(ci_pm_name)));
            } else {
                self.print_with_delay(&format!("   {}", DramaMessages::ci_match().green()));
            }
        }

        // Phase 4: Infrastructure conflict check
        if let (Some(docker_pm_name), Some(ci_pm_name)) = (docker_pm, ci_pm) {
            if docker_pm_name != ci_pm_name {
                thread::sleep(Duration::from_millis(self.delay_ms));
                score += 10;
                self.print_with_delay(&format!(
                    "\n   {}",
                    DramaMessages::infrastructure_war(docker_pm_name, ci_pm_name)
                ));
            }
        }

        // Phase 5: Final calculation
        thread::sleep(Duration::from_millis(self.delay_ms * 2));
        self.print_with_delay(&format!(
            "\n{}",
            Self::random_message(DramaMessages::calculating()).bright_cyan()
        ));

        // Animated progress bar
        self.animate_progress_bar(score);

        // Final reveal
        println!("\n{}", "═".repeat(60).bright_black());
        println!("{}", DramaMessages::final_score(score));
        println!("{}", DramaMessages::score_commentary(score).dimmed());
        println!("{}", "═".repeat(60).bright_black());

        score
    }

    fn animate_progress_bar(&self, score: u8) {
        let bar_width = 40;
        let filled = (score as f32 / 100.0 * bar_width as f32) as usize;

        print!("   [");
        for i in 0..bar_width {
            if i < filled {
                let color = match score {
                    0..=20 => "█".green(),
                    21..=40 => "█".yellow(),
                    41..=60 => "█".bright_yellow(),
                    61..=80 => "█".red(),
                    _ => "█".bright_red(),
                };
                print!("{}", color);
            } else {
                print!("{}", "░".bright_black());
            }
            std::io::Write::flush(&mut std::io::stdout()).ok();
            thread::sleep(Duration::from_millis(20));
        }
        println!("] {}%", score);
    }
}

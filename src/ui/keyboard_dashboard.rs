use crate::match_engine::MatchManager;
use anyhow::Result;
use crossterm::{
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

pub struct KeyboardDashboard {
    match_manager: Arc<MatchManager>,
    current_feedback: Option<String>,
}

impl KeyboardDashboard {
    pub fn new(match_manager: Arc<MatchManager>) -> Self {
        Self {
            match_manager,
            current_feedback: None,
        }
    }

    // Main loop for the dashboard
    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), Clear(ClearType::All), crossterm::cursor::Hide)?;

        info!("Keyboard dashboard started");
        self.render()?; // Initial render

        loop {
            // Handle keyboard input
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if let Some(action) = self.handle_key_event(key) {
                        match action {
                            DashboardAction::ExecuteMatch(match_id) => {
                                self.execute_match(&match_id).await?;
                            }
                            DashboardAction::Quit => {
                                break;
                            }
                        }
                    }
                }
            }
        }

        self.cleanup()?;
        info!("Keyboard dashboard stopped");
        Ok(())
    }

    fn render(&self) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear screen and move to top
        execute!(stdout, Clear(ClearType::All), crossterm::cursor::MoveTo(0, 0))?;

        // Header
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║           Ultra Fast Manual Trading Execution Bot             ║");
        println!("║                     Press 1-9 to Execute                      ║");
        println!("║                          Press Q to Quit                       ║");
        println!("╚════════════════════════════════════════════════════════════════╝");
        println!();

        // Display matches
        let matches = self.match_manager.get_all_matches();
        if matches.is_empty() {
            println!("No matches configured. Please add matches to start trading.");
        } else {
            println!("Active Matches:");
            println!("┌─────┬────────────────────────────────────────────────────────┐");
            println!("│ Key │ Match Name                                           │");
            println!("├─────┼────────────────────────────────────────────────────────┤");

            for (index, engine) in matches.iter().enumerate() {
                let key = (index + 1).to_string();
                let config = engine.get_config();
                let shortcut = config.keyboard_shortcut.map_or(key.clone(), |c| c.to_string());
                
                // Truncate name if too long
                let name = if config.name.len() > 50 {
                    format!("{}...", &config.name[..47])
                } else {
                    config.name.clone()
                };
                
                println!("│  {} │ {:<52} │", shortcut, name);
            }

            println!("└─────┴────────────────────────────────────────────────────────┘");
            println!();
            println!("Press a number key to execute the corresponding match instantly!");
            println!("Execution will send both orders simultaneously for ultra-low latency.");
        }

        println!();
        if let Some(feedback_msg) = &self.current_feedback {
            println!("Status: {}", feedback_msg);
        } else {
            println!("Status: Ready | Latency: < 2ms execution time");
        }

        stdout.flush()?;
        Ok(())
    }

    fn handle_key_event(&self, key: KeyEvent) -> Option<DashboardAction> {
        match key.code {
            KeyCode::Char('q' | 'Q') => Some(DashboardAction::Quit),
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let index = c.to_digit(10).unwrap_or(0) as usize;
                if index > 0 {
                    let matches = self.match_manager.get_all_matches();
                    if index <= matches.len() {
                        let engine = &matches[index - 1];
                        Some(DashboardAction::ExecuteMatch(engine.get_config().id.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            KeyCode::Char(c) => {
                // Check for custom keyboard shortcuts
                let matches = self.match_manager.get_all_matches();
                for engine in matches {
                    if let Some(shortcut) = engine.get_config().keyboard_shortcut {
                        if shortcut == c {
                            return Some(DashboardAction::ExecuteMatch(engine.get_config().id.clone()));
                        }
                    }
                }
                None
            }
            KeyCode::Esc => Some(DashboardAction::Quit),
            _ => None,
        }
    }

    async fn execute_match(&mut self, match_id: &str) -> Result<()> {
        info!("Executing match via keyboard: {}", match_id);
        
        let start_time = std::time::Instant::now();
        
        match self.match_manager.execute_match(match_id).await {
            Ok(_) => {
                let execution_time_ms = start_time.elapsed().as_millis();
                self.set_feedback(format!("✅ SUCCESS! Match: {} | Time: {}ms", match_id, execution_time_ms)); // Use execution_time_ms
            }
            Err(e) => {
                let _execution_time_ms = start_time.elapsed().as_millis(); // Keep for potential future use, or remove if truly not needed
                error!("Match execution failed: {}", e);
                self.set_feedback(format!("❌ FAILED! Match: {} | Error: {}", match_id, e));
            }
        }
        
        self.render()?; // Immediately render to show the feedback
        
        Ok(())
    }

    fn cleanup(&self) -> anyhow::Result<()> {
        let mut stdout = stdout();
        execute!(
            stdout,
            Clear(ClearType::All),
            crossterm::cursor::Show,
            crossterm::cursor::MoveTo(0, 0)
        )?;
        disable_raw_mode()?;
        Ok(())
    }

    fn set_feedback(&mut self, message: String) {
        self.current_feedback = Some(message);
    }
}

#[derive(Debug)]
enum DashboardAction {
    ExecuteMatch(String),
    Quit,
}

use crossterm::event::Event;

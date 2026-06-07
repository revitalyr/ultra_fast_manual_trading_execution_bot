use crate::match_engine::{MatchConfig, MatchManager};
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

// Pure rendering logic - no business logic
pub struct DashboardRenderer;

impl DashboardRenderer {
    pub fn render_dashboard(matches: &[Arc<MatchConfig>], feedback: &Option<String>) -> Result<()> {
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
        if matches.is_empty() {
            println!("No matches configured. Please add matches to start trading.");
        } else {
            println!("Active Matches:");
            println!("┌─────┬────────────────────────────────────────────────────────┐");
            println!("│ Key │ Match Name                                           │");
            println!("├─────┼────────────────────────────────────────────────────────┤");

            for (index, config) in matches.iter().enumerate() {
                let key = (index + 1).to_string();
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
        if let Some(feedback_msg) = feedback {
            println!("Status: {}", feedback_msg);
        } else {
            println!("Status: Ready | Latency: < 2ms execution time");
        }

        stdout.flush()?;
        Ok(())
    }
}

// Pure input processing - no side effects
pub struct InputHandler;

impl InputHandler {
    pub fn handle_key_event(key: KeyEvent, matches: &[Arc<MatchConfig>]) -> Option<DashboardAction> {
        match key.code {
            KeyCode::Char('q' | 'Q') => Some(DashboardAction::Quit),
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let index = c.to_digit(10).unwrap_or(0) as usize;
                if index > 0 && index <= matches.len() {
                    let config = &matches[index - 1];
                    Some(DashboardAction::ExecuteMatch(config.id.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char(c) => {
                // Check for custom keyboard shortcuts
                for config in matches {
                    if let Some(shortcut) = config.keyboard_shortcut {
                        if shortcut == c {
                            return Some(DashboardAction::ExecuteMatch(config.id.clone()));
                        }
                    }
                }
                None
            }
            KeyCode::Esc => Some(DashboardAction::Quit),
            _ => None,
        }
    }
}

// Business logic controller - handles execution
pub struct DashboardController {
    match_manager: Arc<MatchManager>,
}

impl DashboardController {
    pub fn new(match_manager: Arc<MatchManager>) -> Self {
        Self { match_manager }
    }

    pub async fn execute_match(&self, match_id: &str) -> Result<String> {
        info!("Executing match via keyboard: {}", match_id);
        
        let start_time = std::time::Instant::now();
        
        match self.match_manager.execute_match(match_id).await {
            Ok(_) => {
                let execution_time_ms = start_time.elapsed().as_millis();
                Ok(format!("✅ SUCCESS! Match: {} | Time: {}ms", match_id, execution_time_ms))
            }
            Err(e) => {
                error!("Match execution failed: {}", e);
                Ok(format!("❌ FAILED! Match: {} | Error: {}", match_id, e))
            }
        }
    }
}

// Orchestrator - connects UI components
pub struct KeyboardDashboard {
    controller: DashboardController,
    current_feedback: Option<String>,
}

impl KeyboardDashboard {
    pub fn new(match_manager: Arc<MatchManager>) -> Self {
        Self {
            controller: DashboardController::new(match_manager),
            current_feedback: None,
        }
    }

    // Main loop for the dashboard - orchestrates components
    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), Clear(ClearType::All), crossterm::cursor::Hide)?;

        info!("Keyboard dashboard started");
        self.render_initial()?;

        loop {
            // Handle keyboard input
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    let matches = self.get_match_configs();
                    if let Some(action) = InputHandler::handle_key_event(key, &matches) {
                        match action {
                            DashboardAction::ExecuteMatch(match_id) => {
                                self.handle_execute_match(match_id).await?;
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

    fn render_initial(&self) -> Result<()> {
        let matches = self.get_match_configs();
        DashboardRenderer::render_dashboard(&matches, &self.current_feedback)
    }

    fn render(&self) -> Result<()> {
        let matches = self.get_match_configs();
        DashboardRenderer::render_dashboard(&matches, &self.current_feedback)
    }

    fn get_match_configs(&self) -> Vec<Arc<MatchConfig>> {
        self.controller
            .match_manager
            .get_all_matches()
            .iter()
            .map(|engine| Arc::new(engine.get_config().clone()))
            .collect()
    }

    async fn handle_execute_match(&mut self, match_id: String) -> Result<()> {
        let feedback = self.controller.execute_match(&match_id).await?;
        self.set_feedback(feedback);
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
pub enum DashboardAction {
    ExecuteMatch(String),
    Quit,
}

use crossterm::event::Event;

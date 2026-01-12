mod application;
mod domain;
mod infrastructure;
mod ui;

use anyhow::{Context, Result};
use std::io::{self, Write};

use application::{ChallengeRunner, ProgressTracker};
use domain::Challenge;
use infrastructure::{
    AsciinemaRecorder, ChallengeLoader, FileChangeWatcher, HelixEditor, JsonProgressRepository,
    LocalFileSystem, Recorder, TomlChallengeLoader,
};
use ui::{ChallengeListScreen, ChallengeScreen, MainMenuScreen, MenuAction, ProgressScreen, ResultsScreen};

fn main() -> Result<()> {
    // Check if Helix is installed
    if !HelixEditor::is_installed() {
        eprintln!("Error: Helix editor (hx) is not installed or not in PATH.");
        eprintln!("Please install Helix from https://helix-editor.com/");
        std::process::exit(1);
    }

    // Initialize progress tracking
    let progress_repo = JsonProgressRepository::new()
        .context("Failed to initialize progress repository")?;
    let progress_tracker = ProgressTracker::new(progress_repo)
        .context("Failed to load progress")?;

    // Set default editor if not set
    let progress = progress_tracker.get_progress();
    if progress.editor_preference().is_none() {
        progress_tracker.set_editor_preference("Helix".to_string())?;
    }

    // Check if asciinema is installed (optional but recommended)
    let use_recording = check_asciinema()?;

    // Load challenges from TOML files
    let loader = TomlChallengeLoader::new("challenges/helix");
    let challenges = loader.load_all().context("Failed to load challenges")?;
    let total_challenges = challenges.len();

    // Main application loop
    loop {
        let progress = progress_tracker.get_progress();
        let mut main_menu = MainMenuScreen::new();

        let action = main_menu
            .show(&progress, total_challenges)
            .context("Failed to display main menu")?;

        match action {
            MenuAction::StartTraining => {
                if let Err(e) = run_training(&challenges, &progress_tracker, use_recording) {
                    eprintln!("Error during training: {}", e);
                }
            }
            MenuAction::ViewProgress => {
                let progress = progress_tracker.get_progress();
                let progress_screen = ProgressScreen::new();
                progress_screen.show(&progress, total_challenges)
                    .context("Failed to display progress screen")?;
            }
            MenuAction::BrowseChallenges => {
                // Show challenge list without starting one
                let progress = progress_tracker.get_progress();
                let list_screen = ChallengeListScreen::new(challenges.clone())
                    .with_progress(progress);
                let _ = list_screen.show();
            }
            MenuAction::Settings => {
                println!("Settings not yet implemented.");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            MenuAction::Quit => {
                println!("Goodbye!");
                break;
            }
        }
    }

    Ok(())
}

fn check_asciinema() -> Result<bool> {
    if !AsciinemaRecorder::is_available() {
        eprintln!("\n┌─────────────────────────────────────────────┐");
        eprintln!("│           Setup Recommended                 │");
        eprintln!("├─────────────────────────────────────────────┤");
        eprintln!("│                                             │");
        eprintln!("│  asciinema is not installed.                │");
        eprintln!("│                                             │");
        eprintln!("│  Without it, you won't see:                 │");
        eprintln!("│   - Keystroke counts                        │");
        eprintln!("│   - Key sequence feedback                   │");
        eprintln!("│   - Session recordings                      │");
        eprintln!("│                                             │");
        eprintln!("│  Install: https://asciinema.org/docs/       │");
        eprintln!("│                                             │");
        eprintln!("└─────────────────────────────────────────────┘");
        eprint!("\nContinue without recording? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Please install asciinema and try again.");
            std::process::exit(0);
        }
        Ok(false)
    } else {
        Ok(true)
    }
}

fn run_training<R: application::ProgressRepository>(
    challenges: &[Challenge],
    progress_tracker: &ProgressTracker<R>,
    use_recording: bool,
) -> Result<()> {
    let total_challenges = challenges.len();

    // Show challenge list screen with progress
    let progress = progress_tracker.get_progress();
    let list_screen = ChallengeListScreen::new(challenges.to_vec())
        .with_progress(progress);
    let selected_challenge = list_screen
        .show()
        .context("Failed to display challenge list screen")?;

    let challenge = match selected_challenge {
        Some(c) => c,
        None => {
            return Ok(());
        }
    };

    // Show challenge brief screen
    let mut challenge_screen = ChallengeScreen::new();
    let challenge_mode = challenge_screen
        .show(&challenge)
        .context("Failed to display challenge screen")?;

    let challenge_mode = match challenge_mode {
        Some(mode) => mode,
        None => return Ok(()),
    };

    // Dependency injection: create concrete implementations
    let editor = HelixEditor::new();
    let watcher = FileChangeWatcher::new();
    let filesystem = LocalFileSystem::new();

    // Create the challenge runner with injected dependencies
    let mut runner = ChallengeRunner::new(editor, watcher, filesystem);

    // Add recorder if available
    if use_recording {
        let recorder = AsciinemaRecorder::new("hx");
        runner = runner.with_recorder(Box::new(recorder));
    }

    // Run the challenge
    let solution = runner.run(&challenge).context("Failed to run challenge")?;

    // Only record progress and check achievements in challenge mode (not practice mode)
    let newly_unlocked = if !challenge_mode.practice_mode {
        // Record the solution in progress tracker
        progress_tracker
            .record_solution(challenge.id(), &solution)
            .context("Failed to record progress")?;

        // Check for new achievements
        progress_tracker
            .check_achievements(total_challenges)
            .context("Failed to check achievements")?
    } else {
        // Practice mode - no recording
        Vec::new()
    };

    // Show results screen with achievements
    let results_screen = ResultsScreen::new();
    if !newly_unlocked.is_empty() {
        results_screen
            .show_with_achievements(&solution, newly_unlocked)
            .context("Failed to display results screen")?;
    } else {
        results_screen
            .show(&solution)
            .context("Failed to display results screen")?;
    }

    Ok(())
}

//! Git tab: set remote URL, commit message, and push from the project folder.

use crate::theme::{self, TEXT_MUTED};
use egui::{RichText, TextEdit, Ui};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default)]
pub struct GitState {
    pub remote_url: String,
    pub commit_message: String,
    pub status: String,
}

impl GitState {
    pub fn push(&mut self, folder: &Path) -> bool {
        if !folder.is_dir() {
            self.status = "No project folder open.".to_owned();
            return false;
        }

        let message = self.commit_message.trim();
        if message.is_empty() {
            self.status = "Enter a commit message.".to_owned();
            return false;
        }

        if let Err(e) = ensure_gitignore_ignores_target(folder) {
            self.status = e;
            return false;
        }

        if let Err(e) = ensure_git_repo(folder) {
            self.status = e;
            return false;
        }

        let url = self.remote_url.trim();
        if !url.is_empty() {
            if let Err(e) = ensure_remote_origin(folder, url) {
                self.status = e;
                return false;
            }
        } else if !remote_exists(folder, "origin") {
            self.status = "Enter a repository URL (or add an origin remote first).".to_owned();
            return false;
        }

        // Stage everything; `target/` is excluded via .gitignore.
        match run_git(folder, &["add", "."]) {
            Ok(_) => {}
            Err(e) => {
                self.status = format!("git add failed: {e}");
                return false;
            }
        }

        match run_git(folder, &["commit", "-m", message]) {
            Ok(out) => {
                if out.to_lowercase().contains("nothing to commit") {
                    // Still try push in case there are unpushed commits.
                }
            }
            Err(e) => {
                let lower = e.to_lowercase();
                if lower.contains("nothing to commit")
                    || lower.contains("no changes added")
                    || lower.contains("working tree clean")
                {
                    // Continue to push existing commits.
                } else {
                    self.status = format!("git commit failed: {e}");
                    return false;
                }
            }
        }

        match run_git(folder, &["push", "-u", "origin", "HEAD"]) {
            Ok(out) => {
                let summary = out.lines().last().unwrap_or("ok").trim();
                self.status = format!("Push succeeded. {summary}");
                true
            }
            Err(e) => {
                // Fallback without -u in case upstream already set.
                match run_git(folder, &["push"]) {
                    Ok(out) => {
                        let summary = out.lines().last().unwrap_or("ok").trim();
                        self.status = format!("Push succeeded. {summary}");
                        true
                    }
                    Err(e2) => {
                        self.status = format!("git push failed: {e} | {e2}");
                        false
                    }
                }
            }
        }
    }
}

pub fn show(ui: &mut Ui, folder: Option<&Path>, state: &mut GitState) {
    theme::section_title(ui, "Git");

    let Some(dir) = folder.filter(|p| p.is_dir()) else {
        ui.label(
            RichText::new("Open a project folder to use Git.")
                .color(TEXT_MUTED)
                .italics(),
        );
        return;
    };

    ui.label(
        RichText::new(format!("Project: {}", dir.display()))
            .small()
            .monospace()
            .color(TEXT_MUTED),
    );
    ui.add_space(12.0);

    ui.label(RichText::new("Repository URL").small().strong());
    ui.add(
        TextEdit::singleline(&mut state.remote_url)
            .desired_width(f32::INFINITY)
            .hint_text("https://github.com/user/repo.git"),
    );

    ui.add_space(10.0);
    ui.label(RichText::new("Commit message").small().strong());
    ui.add(
        TextEdit::multiline(&mut state.commit_message)
            .desired_width(f32::INFINITY)
            .desired_rows(3)
            .hint_text("Describe your changes"),
    );

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        if theme::primary_button(ui, "↑ Push").clicked() {
            state.push(dir);
        }
        ui.label(
            RichText::new("git add · commit · push  (target/ ignored)")
                .small()
                .color(TEXT_MUTED),
        );
    });

    ui.add_space(8.0);
    if !state.status.is_empty() {
        let color = if state.status.to_lowercase().contains("succeed") {
            egui::Color32::from_rgb(40, 160, 80)
        } else {
            egui::Color32::from_rgb(200, 80, 80)
        };
        ui.colored_label(color, &state.status);
    }
}

fn ensure_gitignore_ignores_target(folder: &Path) -> Result<(), String> {
    let path = folder.join(".gitignore");
    let existing = if path.is_file() {
        std::fs::read_to_string(&path).map_err(|e| format!("Read .gitignore: {e}"))?
    } else {
        String::new()
    };

    let has_target = existing.lines().any(|line| {
        let t = line.trim();
        t == "target" || t == "target/" || t == "/target" || t == "/target/"
    });

    if has_target {
        return Ok(());
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str("# Added by Form Designer — ignore Rust build output\n");
    next.push_str("target/\n");
    std::fs::write(&path, next).map_err(|e| format!("Write .gitignore: {e}"))?;
    Ok(())
}

fn ensure_git_repo(folder: &Path) -> Result<(), String> {
    if folder.join(".git").exists() {
        return Ok(());
    }
    run_git(folder, &["init"]).map(|_| ())
}

fn remote_exists(folder: &Path, name: &str) -> bool {
    run_git(folder, &["remote", "get-url", name]).is_ok()
}

fn ensure_remote_origin(folder: &Path, url: &str) -> Result<(), String> {
    if remote_exists(folder, "origin") {
        run_git(folder, &["remote", "set-url", "origin", url]).map(|_| ())
    } else {
        run_git(folder, &["remote", "add", "origin", url]).map(|_| ())
    }
}

fn run_git(folder: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(folder)
        .output()
        .map_err(|e| format!("failed to start git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}").trim().to_owned();

    if output.status.success() {
        Ok(combined)
    } else {
        Err(if combined.is_empty() {
            format!("exit {}", output.status)
        } else {
            combined
        })
    }
}

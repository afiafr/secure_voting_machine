mod auth;

use arboard::Clipboard;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
    DefaultTerminal, Frame,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, io, time::Duration};
use uuid::Uuid;

const ACTIVE: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;
const HIGHLIGHT: Style = Style::new().fg(Color::Black).bg(Color::Cyan);

#[derive(Clone, Debug)]
struct Candidate {
    name: String,
    party: String,
    votes: u64,
}

#[derive(Clone, Debug)]
struct Position {
    name: String,
    candidates: Vec<Candidate>,
    next_candidate: usize,
}

enum SignupFocus {
    Email,
    PhoneNumber,
    FullName,
    Password,
}

enum LoginFocus {
    Email,
    Password,
}

enum PinFocus {
    Pin,
}

impl Position {
    fn placeholder(n: usize) -> Self {
        Self {
            name: format!("Position {n}"),
            candidates: vec![
                Candidate {
                    name: "Candidate a".into(),
                    party: "Party".into(),
                    votes: 0,
                },
                Candidate {
                    name: "Candidate b".into(),
                    party: "Party".into(),
                    votes: 0,
                },
            ],
            next_candidate: 2,
        }
    }
}

#[derive(Clone, Debug)]
struct Voter {
    email: String,
    full_name: String,
    phone_number: String,
    registered: bool,
    key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Pending,
    Running,
    Finished,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "Not started",
            Self::Running => "Running",
            Self::Finished => "Finished",
        }
    }
}

#[derive(Clone, Debug)]
struct Election {
    id: String,
    nonce: String,
    positions: Vec<Position>,
    voters: Vec<Voter>,
    submitted: HashSet<String>,
    phase: Phase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Main,
    Admin,
    District,
    Vote,
    SystemAdmin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdminFocus {
    Ballot,
    Keys,
    Voters,
}

impl AdminFocus {
    fn next(self) -> Self {
        match self {
            Self::Ballot => Self::Keys,
            Self::Keys => Self::Voters,
            Self::Voters => Self::Ballot,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Ballot => Self::Voters,
            Self::Keys => Self::Ballot,
            Self::Voters => Self::Keys,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DistrictFocus {
    Keys,
    Election,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BallotColumn {
    Position,
    Candidate,
    Party,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BallotCursor {
    row: usize,
    column: BallotColumn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditTarget {
    Position(usize),
    Candidate(usize, usize),
    Party(usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputPurpose {
    Edit(EditTarget),
    ImportVoters,
    NewVoter,
    GenerateKeys,
    EnterElection,
    EnterVotingKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfirmAction {
    ExitAdmin,
    CreateElection,
    StartElection,
    FinishElection,
    SubmitBallot,
}

#[derive(Clone, Debug)]
struct InputDialog {
    title: String,
    purpose: InputPurpose,
    text: String,
    error: Option<String>,
}

#[derive(Clone, Debug)]
enum Modal {
    Input(InputDialog),
    Confirm {
        title: String,
        message: String,
        action: ConfirmAction,
    },
    Info {
        title: String,
        message: String,
        copy_hash: bool,
    },
}

struct App {
    screen: Screen,
    quit: bool,
    main_selected: usize,
    admin_focus: AdminFocus,
    district_focus: DistrictFocus,
    positions: Vec<Position>,
    next_position: usize,
    ballot_cursor: BallotCursor,
    ballot_scroll: usize,
    district_keys: Vec<String>,
    district_key_state: ListState,
    imported_keys: Vec<String>,
    admin_key_state: ListState,
    voters: Vec<Voter>,
    voter_state: ListState,
    election: Option<Election>,
    district_entered: bool,
    district_scroll: usize,
    voting_key: Option<String>,
    voting_choices: Vec<Option<usize>>,
    voting_row: usize,
    voting_scroll: usize,
    modal: Option<Modal>,
    notice: String,
    authenticated: bool,
    login_email: String,
    login_password: String,
    login_focus: LoginFocus,
    signup_focus: SignupFocus,
    conn: Option<Connection>,
    login_phone_number: String,
    login_full_name: String,
    login_role: String,
    voter_pin: String,
    login_pin: String,
    pin_focus: PinFocus,
    }

impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::Main,
            quit: false,
            main_selected: 0,
            admin_focus: AdminFocus::Ballot,
            district_focus: DistrictFocus::Keys,
            positions: (0..3).map(Position::placeholder).collect(),
            next_position: 3,
            ballot_cursor: BallotCursor {
                row: 0,
                column: BallotColumn::Position,
            },
            ballot_scroll: 0,
            district_keys: Vec::new(),
            district_key_state: ListState::default(),
            imported_keys: Vec::new(),
            admin_key_state: ListState::default(),
            voters: Vec::new(),
            voter_state: ListState::default(),
            election: None,
            district_entered: false,
            district_scroll: 0,
            voting_key: None,
            voting_choices: Vec::new(),
            voting_row: 0,
            voting_scroll: 0,
            modal: None,
            notice: "The real security is the friends we make along the way.".into(),
            authenticated: false,
            login_email: String::new(),
            login_password: String::new(),
            login_focus: LoginFocus::Email,
            signup_focus: SignupFocus::Email,
            login_phone_number: String::new(),
            login_full_name: String::new(),
            conn: None,
            login_role: "Administrator".into(),
            voter_pin: String::new(),
            login_pin: String::new(),
            pin_focus: PinFocus::Pin,
        }
    }
}

// Pure navigation/view helper: each candidate occupies one line; a position with
// no candidates still has a selectable position-only line.
fn ballot_rows(positions: &[Position]) -> Vec<(usize, Option<usize>)> {
    let mut rows = Vec::new();
    for (pi, p) in positions.iter().enumerate() {
        if p.candidates.is_empty() {
            rows.push((pi, None));
        } else {
            rows.extend((0..p.candidates.len()).map(|ci| (pi, Some(ci))));
        }
    }
    rows
}

fn letter_suffix(mut index: usize) -> String {
    let mut out = String::new();
    loop {
        out.insert(0, (b'a' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    out
}

// Hook #1: hashes this demo election's shape and its random nonce. This is
// NOT a signature, tamper-proof log, or verification of election integrity.
fn hash_election(election: &Election) -> String {
    let mut sha = Sha256::new();
    sha.update(b"ratatui-election-demo-v1\n");
    sha.update(election.nonce.as_bytes());
    for p in &election.positions {
        sha.update((p.name.len() as u64).to_le_bytes());
        sha.update(p.name.as_bytes());
        for c in &p.candidates {
            sha.update((c.name.len() as u64).to_le_bytes());
            sha.update(c.name.as_bytes());
            sha.update((c.party.len() as u64).to_le_bytes());
            sha.update(c.party.as_bytes());
        }
    }
    format!("{:x}", sha.finalize())
}

// Hook #2: intentionally no outbound communication. Replace with an explicit,
// authenticated transport only after designing a real election backend.
fn send_election(_election: &Election) -> Result<(), String> {
    Ok(())
}

// Demo keys are random identifiers, not cryptographic voting credentials.
fn generate_keys(count: usize) -> Result<Vec<String>, String> {
    if !(1..=10_000).contains(&count) {
        return Err("Enter a number between 1 and 10,000".into());
    }
    Ok((0..count)
        .map(|_| format!("KEY-{}", Uuid::new_v4().simple()))
        .collect())
}

fn start_election(e: &mut Election) -> Result<(), String> {
    if e.phase != Phase::Pending {
        return Err("Election must be waiting to start".into());
    }
    e.phase = Phase::Running;
    Ok(())
}

fn stop_election(e: &mut Election) -> Result<(), String> {
    if e.phase != Phase::Running {
        return Err("Only a running election can be finished".into());
    }
    e.phase = Phase::Finished;
    Ok(())
}

fn submit_ballot(e: &mut Election, key: &str, choices: &[Option<usize>]) -> Result<(), String> {
    if e.phase != Phase::Running {
        return Err("The election is not accepting votes".into());
    }
    if e.submitted.contains(key) {
        return Err("This key has already been used".into());
    }
    if !e.voters.iter().any(|v| v.key.as_deref() == Some(key)) {
        return Err("Key not assigned to this election".into());
    }
    if choices.len() != e.positions.len() || choices.iter().any(Option::is_none) {
        return Err("Choose one candidate for every position".into());
    }
    for (pi, choice) in choices.iter().enumerate() {
        if choice.is_none_or(|ci| ci >= e.positions[pi].candidates.len()) {
            return Err("Ballot contains an invalid candidate".into());
        }
    }
    for (p, selection) in e.positions.iter_mut().zip(choices) {
        p.candidates[selection.expect("validated above")].votes += 1;
    }
    e.submitted.insert(key.to_owned());
    Ok(())
}


fn draw_pin(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(2),
    ])
    .margin(1)
    .split(area);

    frame.render_widget(
        Paragraph::new(format!("Enter your {} PIN number", app.login_role))
            .alignment(Alignment::Center),
        sections[0],
    );

    let pin_focused = matches!(app.pin_focus, PinFocus::Pin);

    // Never display the actual PIN.
    let masked_pin = "*".repeat(app.login_pin.chars().count());

    let pin_block = Block::bordered()
        .title_top(if pin_focused {
            "PIN • typing"
        } else {
            "PIN"
        })
        .border_style(
            Style::new().fg(if pin_focused { ACTIVE } else { MUTED }),
        );

    frame.render_widget(
        Paragraph::new(masked_pin).block(pin_block),
        sections[1],
    );

    frame.render_widget(
        Paragraph::new(
            "Tab: switch field | Enter: sign in | Esc: return\n\
             Voter access requires successful PIN verification.",
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::new().fg(Color::Gray)),
        sections[2],
    );
}


fn draw_login(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(2),
    ])
    .margin(1)
    .split(area);

    frame.render_widget(
        Paragraph::new(format!("Sign in to your {} account", app.login_role))
            .alignment(Alignment::Center),
        sections[0],
    );

    let email_focused = matches!(app.login_focus, LoginFocus::Email);
    let password_focused = matches!(app.login_focus, LoginFocus::Password);

    let email_block = Block::bordered()
        .title_top(if email_focused {
            "Email • typing"
        } else {
            "Email"
        })
        .border_style(Style::new().fg(if email_focused { ACTIVE } else { MUTED }));

    frame.render_widget(
        Paragraph::new(app.login_email.as_str()).block(email_block),
        sections[1],
    );

    // Never display the actual password.
    let masked_password = "*".repeat(app.login_password.chars().count());

    let password_block = Block::bordered()
        .title_top(if password_focused {
            "Password • typing"
        } else {
            "Password"
        })
        .border_style(Style::new().fg(if password_focused { ACTIVE } else { MUTED }));

    frame.render_widget(
        Paragraph::new(masked_password).block(password_block),
        sections[2],
    );

    frame.render_widget(
        Paragraph::new(
            "Tab: switch field | Enter: sign in | Esc: return\n\
             Administrator access requires successful verification.",
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::new().fg(Color::Gray)),
        sections[3],
    );
}

impl App {
    fn info(&mut self, title: &str, message: impl Into<String>) {
        self.modal = Some(Modal::Info {
            title: title.into(),
            message: message.into(),
            copy_hash: false,
        });
    }

    fn input(&mut self, title: &str, purpose: InputPurpose, text: impl Into<String>) {
        self.modal = Some(Modal::Input(InputDialog {
            title: title.into(),
            purpose,
            text: text.into(),
            error: None,
        }));
    }

    fn confirm(&mut self, title: &str, message: &str, action: ConfirmAction) {
        self.modal = Some(Modal::Confirm {
            title: title.into(),
            message: message.into(),
            action,
        });
    }

    fn rows(&self) -> Vec<(usize, Option<usize>)> {
        ballot_rows(&self.positions)
    }

    fn current_cell(&self) -> Option<EditTarget> {
        let (pi, maybe_ci) = *self.rows().get(self.ballot_cursor.row)?;
        match self.ballot_cursor.column {
            BallotColumn::Position => Some(EditTarget::Position(pi)),
            BallotColumn::Candidate => maybe_ci.map(|ci| EditTarget::Candidate(pi, ci)),
            BallotColumn::Party => maybe_ci.map(|ci| EditTarget::Party(pi, ci)),
        }
    }

    fn clamp_cursor(&mut self) {
        let rows = self.rows();
        self.ballot_cursor.row = self.ballot_cursor.row.min(rows.len().saturating_sub(1));
        if rows
            .get(self.ballot_cursor.row)
            .is_some_and(|(_, c)| c.is_none())
        {
            self.ballot_cursor.column = BallotColumn::Position;
        }
        if self.ballot_cursor.column == BallotColumn::Position
            && rows
                .get(self.ballot_cursor.row)
                .is_some_and(|(_, c)| *c != Some(0))
        {
            if let Some((pi, _)) = rows.get(self.ballot_cursor.row) {
                self.ballot_cursor.row = rows.iter().position(|(p, _)| p == pi).unwrap_or(0);
            }
        }
    }

    fn move_ballot(&mut self, key: KeyCode) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        match key {
            KeyCode::Up | KeyCode::Down => {
                if self.ballot_cursor.column == BallotColumn::Position {
                    let pi = rows[self.ballot_cursor.row].0;
                    let target_pi = if key == KeyCode::Up {
                        pi.saturating_sub(1)
                    } else {
                        (pi + 1).min(self.positions.len() - 1)
                    };
                    self.ballot_cursor.row =
                        rows.iter().position(|(p, _)| *p == target_pi).unwrap_or(0);
                } else {
                    self.ballot_cursor.row = if key == KeyCode::Up {
                        self.ballot_cursor.row.saturating_sub(1)
                    } else {
                        (self.ballot_cursor.row + 1).min(rows.len() - 1)
                    };
                }
            }
            KeyCode::Left => {
                self.ballot_cursor.column = match self.ballot_cursor.column {
                    BallotColumn::Party => BallotColumn::Candidate,
                    BallotColumn::Candidate => {
                        let pi = rows[self.ballot_cursor.row].0;
                        self.ballot_cursor.row =
                            rows.iter().position(|(p, _)| *p == pi).unwrap_or(0);
                        BallotColumn::Position
                    }
                    BallotColumn::Position => BallotColumn::Position,
                };
            }
            KeyCode::Right => {
                if rows[self.ballot_cursor.row].1.is_some() {
                    self.ballot_cursor.column = match self.ballot_cursor.column {
                        BallotColumn::Position => BallotColumn::Candidate,
                        BallotColumn::Candidate | BallotColumn::Party => BallotColumn::Party,
                    };
                }
            }
            _ => {}
        }
        self.clamp_cursor();
    }

    fn add_position(&mut self) {
        let id = self.next_position;
        self.next_position += 1;
        self.positions.push(Position::placeholder(id));
        let pi = self.positions.len() - 1;
        self.ballot_cursor = BallotCursor {
            row: self.rows().iter().position(|(p, _)| *p == pi).unwrap_or(0),
            column: BallotColumn::Position,
        };
    }

    fn add_candidate(&mut self) {
        let Some((pi, _)) = self.rows().get(self.ballot_cursor.row).copied() else {
            return;
        };
        let p = &mut self.positions[pi];
        let name = format!("Candidate {}", letter_suffix(p.next_candidate));
        p.next_candidate += 1;
        p.candidates.push(Candidate {
            name,
            party: "Party".into(),
            votes: 0,
        });
        let ci = p.candidates.len() - 1;
        self.ballot_cursor = BallotCursor {
            row: self
                .rows()
                .iter()
                .position(|r| *r == (pi, Some(ci)))
                .unwrap_or(0),
            column: BallotColumn::Candidate,
        };
    }

    fn delete_cell(&mut self) {
        match self.current_cell() {
            Some(EditTarget::Position(pi)) => {
                self.positions.remove(pi);
            }
            Some(EditTarget::Candidate(pi, ci)) | Some(EditTarget::Party(pi, ci)) => {
                self.positions[pi].candidates.remove(ci);
            }
            None => {}
        }
        self.clamp_cursor();
    }

    fn edit_cell(&mut self) {
        let Some(target) = self.current_cell() else {
            return;
        };
        let text = match target {
            EditTarget::Position(pi) => self.positions[pi].name.clone(),
            EditTarget::Candidate(pi, ci) => self.positions[pi].candidates[ci].name.clone(),
            EditTarget::Party(pi, ci) => self.positions[pi].candidates[ci].party.clone(),
        };
        self.input("Edit", InputPurpose::Edit(target), text);
    }

    fn import_keys(&mut self) {
        let mut existing: HashSet<_> = self.imported_keys.iter().cloned().collect();
        let mut count = 0;
        for key in &self.district_keys {
            if existing.insert(key.clone()) {
                self.imported_keys.push(key.clone());
                count += 1;
            }
        }
        if !self.imported_keys.is_empty() && self.admin_key_state.selected().is_none() {
            self.admin_key_state.select(Some(0));
        }
        self.notice = format!("Imported {count} new key(s) from District Official");
    }

    fn unassign_key(&mut self) {
        let Some(index) = self.admin_key_state.selected() else {
            return;
        };
        let Some(key) = self.imported_keys.get(index) else {
            return;
        };
        if let Some(voter) = self
            .voters
            .iter_mut()
            .find(|v| v.key.as_deref() == Some(key))
        {
            voter.key = None;
            self.notice = "Key unassigned; voter is now unchecked".into();
        } else {
            self.notice = "Selected key is already unassigned".into();
        }
    }

    fn add_voters(&mut self, contacts: Vec<Voter>) -> usize {
        let mut seen: HashSet<_> = self
            .voters
            .iter()
            .map(|v| v.email.to_ascii_lowercase())
            .collect();
        let mut n = 0;
        for voter in contacts {
            let email = voter.email.trim().trim_matches('"').trim().to_string();
            if email.is_empty() || email.eq_ignore_ascii_case("contact") {
                continue;
            }
            if seen.insert(email.to_ascii_lowercase()) {
                self.voters.push(Voter {
                    email: email.clone(),
                    full_name: voter.full_name.clone(),
                    phone_number: voter.phone_number.clone(),
                    registered: false,
                    key: None,
                });
                n += 1;
            }

            let _ = auth::login::register_voter_pin(self.conn.as_ref().unwrap(), &email, &voter.full_name, &voter.phone_number);
        }
        if !self.voters.is_empty() && self.voter_state.selected().is_none() {
            self.voter_state.select(Some(0));
        }
        n
    }

    fn assign_keys(&mut self) -> Result<(), String> {
        let allocated: HashSet<_> = self.voters.iter().filter_map(|v| v.key.clone()).collect();
        let available: Vec<_> = self
            .imported_keys
            .iter()
            .filter(|k| !allocated.contains(*k))
            .cloned()
            .collect();
        let needed = self.voters.iter().filter(|v| v.key.is_none()).count();
        if needed > available.len() {
            return Err(format!(
                "Need {needed} unassigned key(s), but only {} available. Import more first.",
                available.len()
            ));
        }
        let mut pool = available.into_iter();
        for voter in &mut self.voters {
            if voter.key.is_none() {
                voter.key = pool.next();
            }
        }
        Ok(())
    }

    fn validate_election(&self) -> Result<(), String> {
        if self
            .election
            .as_ref()
            .is_some_and(|e| e.phase == Phase::Running)
        {
            return Err("Finish the existing election before replacing it".into());
        }
        if self.positions.is_empty() {
            return Err("Add at least one position".into());
        }
        if self
            .positions
            .iter()
            .any(|p| p.name.trim().is_empty() || p.candidates.is_empty())
        {
            return Err("Each position needs a name and at least one candidate".into());
        }
        if self
            .positions
            .iter()
            .flat_map(|p| &p.candidates)
            .any(|c| c.name.trim().is_empty() || c.party.trim().is_empty())
        {
            return Err("Every candidate needs a name and exactly one nonempty party field".into());
        }
        if self.voters.is_empty() {
            return Err("Add at least one voter".into());
        }
        if self.voters.iter().any(|v| v.key.is_none()) {
            return Err(
                "Every voter must have exactly one assigned key. Use (K) Assign Keys.".into(),
            );
        }
        let unique: HashSet<_> = self
            .voters
            .iter()
            .filter_map(|v| v.key.as_deref())
            .collect();
        if unique.len() != self.voters.len() {
            return Err("Each voter must have a different key".into());
        }
        Ok(())
    }

    fn create_election(&mut self) -> Result<(), String> {
        self.validate_election()?;
        let mut e = Election {
            id: String::new(),
            nonce: Uuid::new_v4().to_string(),
            positions: self.positions.clone(),
            voters: self.voters.clone(),
            submitted: HashSet::new(),
            phase: Phase::Pending,
        };
        e.id = hash_election(&e);
        send_election(&e)?;
        let hash = e.id.clone();
        self.election = Some(e);
        self.district_entered = false;
        self.voting_key = None;
        self.voting_choices.clear();
        self.modal = Some(Modal::Info {
            title: "Election Created (Local Demo)".into(),
            message: hash,
            copy_hash: true,
        });
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return;
        }
        if self.modal.is_some() {
            self.handle_modal(key);
            return;
        }
        match self.screen {
            Screen::Main => self.handle_main(key),
            Screen::SystemAdmin => self.handle_system_admin(key),
            Screen::Admin => self.handle_admin(key),
            Screen::District => self.handle_district(key),
            Screen::Vote => self.handle_vote(key),
        }
    }

    fn handle_main(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.main_selected = self.main_selected.saturating_sub(1),
            KeyCode::Down => self.main_selected = (self.main_selected + 1).min(3),
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.screen = match self.main_selected {
                    0 => Screen::SystemAdmin,
                    1 => Screen::Admin,
                    2 => Screen::District,
                    _ => Screen::Vote,
                };
            }
            KeyCode::Char('q' | 'Q') | KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    fn submit_signup(&mut self) {
        let Some(conn) = self.conn.as_ref() else {
            self.info("Sign up Error", "Database connection is unavailable.");
            return;
        };

        let result = crate::auth::login::register_user(
            conn,
            self.login_email.trim(),
            self.login_phone_number.trim(),
            self.login_role.trim(),
            self.login_full_name.trim(),
            &self.login_password,
        );

        self.login_password.clear();

        match result {
            Ok(true) => {
                // Password verified, but admin access is NOT granted yet.
                //
                // Next:
                // 1. Check this user's stored database role is Admin.
                // 2. Complete OTP verification if required.
                // 3. Only after both succeed:
                //    self.authenticated = true;

                self.confirm(
                    "Account successfully created",
                    "Return to the main menu?",
                    ConfirmAction::ExitAdmin,
                );

            }

            Ok(false) => {
                self.info(
                    "Sign up Failed",
                    "Unable to create your account. Please try again.",
                );
            }

            Err(err) => {
                self.info("Sign Up Error", err.to_string());
            }
        }
    }

    fn submit_login(&mut self) {
        let Some(conn) = self.conn.as_ref() else {
            self.info("Login Error", "Database connection is unavailable.");
            return;
        };

        let result =
            crate::auth::login::handle_login(conn, self.login_email.trim(), &self.login_password);

        self.login_password.clear();

        match result {
            Ok(true) => {
                // Password verified, but admin access is NOT granted yet.
                //
                // Next:
                // 1. Check this user's stored database role is Admin.
                // 2. Complete OTP verification if required.
                // 3. Only after both succeed:
                //    self.authenticated = true;

                self.authenticated = true;
            }

            Ok(false) => {
                self.info("Login Failed", "Invalid email or password.");
            }

            Err(_) => {
                self.info(
                    "Login Error",
                    "Unable to verify your login. Please try again.",
                );
            }
        }
    }

    fn handle_login(&mut self, key: KeyEvent) {
        // Ignore key-release events.
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                self.login_focus = match self.login_focus {
                    LoginFocus::Email => LoginFocus::Password,
                    LoginFocus::Password => LoginFocus::Email,
                };
            }

            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match self.login_focus {
                    LoginFocus::Email => self.login_email.push(c),
                    LoginFocus::Password => self.login_password.push(c),
                }
            }

            KeyCode::Backspace => match self.login_focus {
                LoginFocus::Email => {
                    self.login_email.pop();
                }
                LoginFocus::Password => {
                    self.login_password.pop();
                }
            },

            KeyCode::Enter => self.submit_login(),

            KeyCode::Esc => {
                self.login_password.clear();
                self.confirm(
                    "Leave Admin Login?",
                    "Return to the main menu?",
                    ConfirmAction::ExitAdmin,
                );
            }

            _ => {}
        }
    }

    fn submit_pin(&mut self) {
        let Some(conn) = self.conn.as_ref() else {
            self.info("Login Error", "Database connection is unavailable.");
            return;
        };

        let result = crate::auth::login::handle_pin(
            conn,
            &self.voter_pin,
        );

        self.voter_pin.clear();

        match result {
            Ok(true) => {
                // Password verified, but admin access is NOT granted yet.
                //
                // Next:
                // 1. Check this user's stored database role is Admin.
                // 2. Complete OTP verification if required.
                // 3. Only after both succeed:
                //    self.authenticated = true;

                self.authenticated = true;
            }

            Ok(false) => {
                self.info("Wrong PIN", "Invalid PIN.");
            }

            Err(_) => {
                self.info(
                    "Pin Error",
                    "Unable to verify your PIN. Please try again.",
                );
            }
        }
    }

    fn handle_pin(&mut self, key: KeyEvent) {
        // Ignore key-release events.
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char(c)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT,
                ) =>
            {
                self.voter_pin.push(c);
            }

            KeyCode::Backspace => {
                self.voter_pin.pop();
            }

            KeyCode::Enter => self.submit_pin(),

            KeyCode::Esc => {
                self.voter_pin.clear();
                self.confirm(
                    "Leave Voter Login?",
                    "Return to the main menu?",
                    ConfirmAction::ExitAdmin,
                );
            }

            _ => {}
        }
    }

    fn handle_system_admin(&mut self, key: KeyEvent) {
        // Ignore key-release events.
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                self.signup_focus = match self.signup_focus {
                    SignupFocus::Email => SignupFocus::Password,
                    SignupFocus::Password => SignupFocus::PhoneNumber,
                    SignupFocus::PhoneNumber => SignupFocus::FullName,
                    SignupFocus::FullName => SignupFocus::Email,
                };
            }

            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match self.signup_focus {
                    SignupFocus::Email => self.login_email.push(c),
                    SignupFocus::PhoneNumber => self.login_phone_number.push(c),
                    SignupFocus::Password => self.login_password.push(c),
                    SignupFocus::FullName => self.login_full_name.push(c),
                }
            }

            KeyCode::Backspace => match self.signup_focus {
                SignupFocus::Email => {
                    self.login_email.pop();
                }
                SignupFocus::PhoneNumber => {
                    self.login_phone_number.pop();
                }
                SignupFocus::FullName => {
                    self.login_full_name.pop();
                }
                SignupFocus::Password => {
                    self.login_password.pop();
                }
            },

            KeyCode::Enter => self.submit_signup(),

            KeyCode::Esc => {
                self.login_password.clear();
                self.confirm(
                    "Leave Admin Signup?",
                    "Return to the main menu?",
                    ConfirmAction::ExitAdmin,
                );
            }

            _ => {}
        }
    }

    fn handle_admin(&mut self, key: KeyEvent) {
        // While unauthenticated, send all input to the login form.
        if !self.authenticated {
            self.handle_login(key);
            return;
        }

        // Some terminal emulators cannot distinguish Shift+Enter from Enter.
        if key.code == KeyCode::Enter
            && key
                .modifiers
                .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL)
        {
            if let Err(err) = self.validate_election() {
                self.info("Cannot Create Election", err);
            } else {
                self.confirm(
                    "Create Election?",
                    "Create this election with the current ballot, voters, and assigned keys?",
                    ConfirmAction::CreateElection,
                );
            }
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.confirm(
                    "Leave Election Administrator?",
                    "Return to the main menu? The local draft is kept.",
                    ConfirmAction::ExitAdmin,
                );
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.admin_focus = if key.code == KeyCode::BackTab
                    || key.modifiers.contains(KeyModifiers::SHIFT)
                {
                    self.admin_focus.prev()
                } else {
                    self.admin_focus.next()
                };
            }
            _ => match self.admin_focus {
                AdminFocus::Ballot => match key.code {
                    KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                        self.move_ballot(key.code)
                    }
                    KeyCode::Char('p' | 'P') => self.add_position(),
                    KeyCode::Char('c' | 'C') => self.add_candidate(),
                    KeyCode::Char('d' | 'D') => self.delete_cell(),
                    KeyCode::Char('e' | 'E') => self.edit_cell(),
                    _ => {}
                },
                AdminFocus::Keys => match key.code {
                    KeyCode::Up | KeyCode::Down => move_selection(
                        &mut self.admin_key_state,
                        self.imported_keys.len(),
                        key.code,
                    ),
                    KeyCode::Char('i' | 'I') => self.import_keys(),
                    KeyCode::Char('u' | 'U') => self.unassign_key(),
                    _ => {}
                },
                AdminFocus::Voters => match key.code {
                    KeyCode::Up | KeyCode::Down => {
                        move_selection(&mut self.voter_state, self.voters.len(), key.code)
                    }
                    KeyCode::Char('o' | 'O') => {
                        self.input("Open Voter File", InputPurpose::ImportVoters, "")
                    }
                    KeyCode::Char('n' | 'N') => self.input("New Voter", InputPurpose::NewVoter, ""),
                    KeyCode::Char('r' | 'R') => {
                        if let Some(i) = self.voter_state.selected() {
                            if let Some(v) = self.voters.get_mut(i) {
                                v.registered = true;
                                self.notice = format!("Registered {} (local demo)", v.full_name);
                            }
                        }
                    }
                    KeyCode::Char('a' | 'A') => {
                        for voter in &mut self.voters {
                            voter.registered = true;
                        }
                        self.notice =
                            format!("Registered {} voter(s) (local demo)", self.voters.len());
                    }
                    KeyCode::Char('k' | 'K') => match self.assign_keys() {
                        Ok(()) => self.notice = "Assigned one unique key per voter".into(),
                        Err(e) => self.info("Cannot Assign Keys", e),
                    },
                    _ => {}
                },
            },
        }
    }

    fn handle_district(&mut self, key: KeyEvent) {
        if !self.authenticated {
            self.handle_login(key);
            return;
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                self.district_focus = if self.district_focus == DistrictFocus::Keys {
                    DistrictFocus::Election
                } else {
                    DistrictFocus::Keys
                };
            }
            _ => match self.district_focus {
                DistrictFocus::Keys => match key.code {
                    KeyCode::Up | KeyCode::Down => move_selection(
                        &mut self.district_key_state,
                        self.district_keys.len(),
                        key.code,
                    ),
                    KeyCode::Char('g' | 'G') => {
                        self.input("Generate Keys", InputPurpose::GenerateKeys, "")
                    }
                    KeyCode::Char('c' | 'C') => {
                        self.district_keys.clear();
                        self.district_key_state.select(None);
                        self.notice = "Cleared generated-key list (previous imports remain)".into();
                    }
                    _ => {}
                },
                DistrictFocus::Election => match key.code {
                    KeyCode::Up => self.district_scroll = self.district_scroll.saturating_sub(1),
                    KeyCode::Down => self.district_scroll += 1,
                    KeyCode::Char('e' | 'E') => {
                        self.input("Enter Election Hash", InputPurpose::EnterElection, "")
                    }
                    KeyCode::Char('s' | 'S') => {
                        if self.district_entered
                            && self
                                .election
                                .as_ref()
                                .is_some_and(|e| e.phase == Phase::Pending)
                        {
                            self.confirm(
                                "Start Election?",
                                "Open this election for voting?",
                                ConfirmAction::StartElection,
                            );
                        } else {
                            self.info(
                                "Cannot Start Election",
                                "Enter a pending election hash first.",
                            );
                        }
                    }
                    KeyCode::Char('f' | 'F') => {
                        if self.district_entered
                            && self
                                .election
                                .as_ref()
                                .is_some_and(|e| e.phase == Phase::Running)
                        {
                            self.confirm(
                                "Finish Election?",
                                "Close voting and publish local tallies?",
                                ConfirmAction::FinishElection,
                            );
                        } else {
                            self.info("Cannot Finish Election", "Enter a running election first.");
                        }
                    }
                    KeyCode::Char('r' | 'R') => match self.election.as_ref() {
                        Some(e) if self.district_entered && e.phase == Phase::Finished => {
                            self.district_scroll = 0;
                            self.notice = "Final tallies appear in Run Election".into();
                        }
                        _ => self.info(
                            "Results Unavailable",
                            "Results are only shown when an entered election has finished.",
                        ),
                    },
                    _ => {}
                },
            },
        }
    }

    fn handle_vote(&mut self, key: KeyEvent) {
        if !self.authenticated {
            self.handle_pin(key);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::Main;
                self.voting_key = None;
            }
            KeyCode::Char('e' | 'E') => {
                self.input("Enter Voting Key", InputPurpose::EnterVotingKey, "")
            }
            KeyCode::Char('v' | 'V') => {
                if self
                    .election
                    .as_ref()
                    .is_some_and(|e| e.phase == Phase::Finished)
                {
                    self.voting_scroll = 0;
                    self.notice = "Final vote totals are shown beside each candidate".into();
                } else {
                    self.info(
                        "Results Unavailable",
                        "Results are hidden until the election has finished.",
                    );
                }
            }
            KeyCode::Char('s' | 'S') => {
                match self.election.as_ref().map(|e| e.phase) {
                    Some(Phase::Running) => {
                        if self.voting_key.is_none() {
                            self.info(
                                "No Voting Key",
                                "Press E and enter a valid assigned key first.",
                            );
                        } else if self.voting_choices.iter().any(Option::is_none) {
                            self.info(
                                "Incomplete Ballot",
                                "Select one candidate for every position.",
                            );
                        } else {
                            self.confirm("Submit Ballot?", "Cast your selected votes? You cannot submit twice with the same key.", ConfirmAction::SubmitBallot);
                        }
                    }
                    Some(Phase::Finished) => self.info(
                        "Election Closed",
                        "Sorry, you missed your chance to participate in democracy",
                    ),
                    _ => self.info(
                        "Election Not Started",
                        "Sorry, that election has not began yet",
                    ),
                }
            }
            KeyCode::Up => self.voting_row = self.voting_row.saturating_sub(1),
            KeyCode::Down => {
                if let Some(e) = &self.election {
                    let n = ballot_rows(&e.positions).len();
                    self.voting_row = (self.voting_row + 1).min(n.saturating_sub(1));
                }
            }
            KeyCode::Char(' ') => {
                if let (Some(e), Some(key)) = (&self.election, &self.voting_key) {
                    if e.phase != Phase::Running {
                        return;
                    }
                    if e.submitted.contains(key) {
                        self.info(
                            "Already Voted",
                            "This key has already submitted its ballot.",
                        );
                        return;
                    }
                    if let Some((pi, Some(ci))) =
                        ballot_rows(&e.positions).get(self.voting_row).copied()
                    {
                        self.voting_choices[pi] = if self.voting_choices[pi] == Some(ci) {
                            None
                        } else {
                            Some(ci)
                        };
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_modal(&mut self, key: KeyEvent) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        match modal {
            Modal::Input(mut dialog) => match key.code {
                KeyCode::Esc => {}
                KeyCode::Backspace => {
                    dialog.text.pop();
                    self.modal = Some(Modal::Input(dialog));
                }
                KeyCode::Enter => {
                    if let Err(err) =
                        self.finish_input(dialog.purpose, dialog.text.trim().to_string())
                    {
                        dialog.error = Some(err);
                        self.modal = Some(Modal::Input(dialog));
                    }
                }
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    dialog.text.push(ch);
                    self.modal = Some(Modal::Input(dialog));
                }
                _ => {
                    self.modal = Some(Modal::Input(dialog));
                }
            },
            Modal::Confirm {
                title,
                message,
                action,
            } => match key.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                    if let Err(err) = self.finish_confirm(action) {
                        self.info("Action Failed", err);
                    }
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {}
                _ => {
                    self.modal = Some(Modal::Confirm {
                        title,
                        message,
                        action,
                    })
                }
            },
            Modal::Info {
                title,
                message,
                copy_hash,
            } => {
                if copy_hash && matches!(key.code, KeyCode::Char('c' | 'C')) {
                    match Clipboard::new().and_then(|mut cb| cb.set_text(message.clone())) {
                        Ok(()) => self.notice = "Election hash copied to system clipboard".into(),
                        Err(e) => self.notice = format!("Clipboard unavailable: {e}"),
                    }
                    self.modal = Some(Modal::Info {
                        title,
                        message,
                        copy_hash,
                    });
                } else if copy_hash && key.code == KeyCode::Esc {
                    self.screen = Screen::Main;
                } else if !copy_hash
                    && matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' '))
                {
                    // Close normal info/error dialogs.
                } else {
                    self.modal = Some(Modal::Info {
                        title,
                        message,
                        copy_hash,
                    });
                }
            }
        }
    }

    fn finish_input(&mut self, purpose: InputPurpose, text: String) -> Result<(), String> {
        fn parse_voter_input(text: &str) -> Result<Voter, String> {
            let fields: Vec<&str> = text.split(',').map(str::trim).collect();

            if fields.len() != 3 || fields.iter().any(|field| field.is_empty()) {
                return Err(
                    "Expected: email,full_name,phone_number".into(),
                );
            }

            Ok(Voter {
                email: fields[0].to_string(),
                full_name: fields[1].to_string(),
                phone_number: fields[2].to_string(),
                registered: false,
                key: None,
            })
        }
        match purpose {
            InputPurpose::Edit(target) => {
                if text.is_empty() {
                    return Err("Field cannot be empty".into());
                }
                match target {
                    EditTarget::Position(pi) => {
                        self.positions.get_mut(pi).ok_or("Position not found")?.name = text
                    }
                    EditTarget::Candidate(pi, ci) => {
                        self.positions
                            .get_mut(pi)
                            .and_then(|p| p.candidates.get_mut(ci))
                            .ok_or("Candidate not found")?
                            .name = text
                    }
                    EditTarget::Party(pi, ci) => {
                        self.positions
                            .get_mut(pi)
                            .and_then(|p| p.candidates.get_mut(ci))
                            .ok_or("Party not found")?
                            .party = text
                    }
                }
                self.notice = "Ballot field updated".into();
            }
            InputPurpose::ImportVoters => {
                let path = text.trim();

                let contents = fs::read_to_string(path)
                    .map_err(|e| format!("Cannot read {path}: {e}"))?;

                let contacts: Vec<Voter> = contents
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| !line.trim().is_empty())
                    .map(|(index, line)| {
                        parse_voter_input(line)
                            .map_err(|e| format!("Line {}: {e}", index + 1))
                    })
                    .collect::<Result<Vec<_>, String>>()?;

                let count = self.add_voters(contacts);
                self.notice = format!("Imported {count} new voter(s) from {path}");
            }

            InputPurpose::NewVoter => {
                let voter = parse_voter_input(&text)?;
                let added = self.add_voters(vec![voter]);

                self.notice = if added == 0 {
                    "Voter already exists".into()
                } else {
                    "Added voter".into()
                };
            }
            InputPurpose::GenerateKeys => {
                let count: usize = text.parse().map_err(|_| "Enter an integer".to_string())?;
                let keys = generate_keys(count)?;
                self.district_keys.extend(keys);
                self.district_key_state
                    .select(Some(self.district_keys.len() - count));
                self.notice = format!("Generated {count} local demo key(s)");
            }
            InputPurpose::EnterElection => match &self.election {
                Some(e) if e.id == text => {
                    self.district_entered = true;
                    self.district_scroll = 0;
                    self.notice = format!("Entered election: {}", e.phase.label());
                }
                _ => return Err("No local election matches that hash".into()),
            },
            InputPurpose::EnterVotingKey => {
                let e = self
                    .election
                    .as_ref()
                    .ok_or("No election has been created")?;
                if !e
                    .voters
                    .iter()
                    .any(|v| v.key.as_deref() == Some(text.as_str()))
                {
                    return Err("This key is not assigned to the current election".into());
                }
                if e.submitted.contains(&text) {
                    return Err("This key has already submitted a ballot".into());
                }
                self.voting_key = Some(text);
                self.voting_choices = vec![None; e.positions.len()];
                self.voting_row = 0;
                self.voting_scroll = 0;
                self.notice = format!("Election entered: {}", e.phase.label());
            }
        }
        Ok(())
    }

    fn finish_confirm(&mut self, action: ConfirmAction) -> Result<(), String> {
        match action {
            ConfirmAction::ExitAdmin => {
                self.authenticated = false;
                self.login_email.clear();
                self.login_password.clear();
                self.login_focus = LoginFocus::Email;
                self.screen = Screen::Main;
            }
            ConfirmAction::CreateElection => self.create_election()?,
            ConfirmAction::StartElection => {
                let e = self
                    .election
                    .as_mut()
                    .ok_or("No election has been created")?;
                start_election(e)?;
                self.notice = "Election started (local demo)".into();
            }
            ConfirmAction::FinishElection => {
                let e = self
                    .election
                    .as_mut()
                    .ok_or("No election has been created")?;
                stop_election(e)?;
                self.notice = "Election finished. Final tallies are now visible.".into();
            }
            ConfirmAction::SubmitBallot => {
                let key = self.voting_key.as_deref().ok_or("No voting key entered")?;
                let e = self
                    .election
                    .as_mut()
                    .ok_or("No election has been created")?;
                submit_ballot(e, key, &self.voting_choices)?;
                self.voting_key = None;
                self.voting_choices.clear();
                self.info(
                    "Ballot Submitted",
                    "Your demo ballot was counted locally. This is not a real voting receipt.",
                );
            }
        }
        Ok(())
    }
}

fn move_selection(state: &mut ListState, len: usize, key: KeyCode) {
    if len == 0 {
        state.select(None);
        return;
    }
    let at = state.selected().unwrap_or(0).min(len - 1);
    state.select(Some(if key == KeyCode::Up {
        at.saturating_sub(1)
    } else {
        (at + 1).min(len - 1)
    }));
}

fn center(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_pct) / 2),
            Constraint::Percentage(height_pct),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_pct) / 2),
            Constraint::Percentage(width_pct),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

fn panel(title: &'static str, footer: impl Into<Line<'static>>, focused: bool) -> Block<'static> {
    Block::bordered()
        .title_top(title)
        .title_bottom(footer)
        .border_style(if focused {
            Style::new().fg(ACTIVE)
        } else {
            Style::new().fg(MUTED)
        })
}

fn headline(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(
        Paragraph::new(Line::from(text.to_owned()).centered())
            .style(Style::new().fg(Color::White).add_modifier(Modifier::BOLD)),
        area,
    );
}

fn draw(frame: &mut Frame, app: &mut App) {
    let bounds = frame.area();
    let page = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(bounds);
    let title = match app.screen {
        Screen::Main => "(Secure) VOTING MACHINE",
        Screen::SystemAdmin => "System Administrator",
        Screen::Admin => "Election Administrator",
        Screen::District => "District Official",
        Screen::Vote => "Vote!",
    };
    headline(frame, page[0], title);
    match app.screen {
        Screen::Main => draw_main(frame, page[1], app),
        Screen::SystemAdmin => draw_system_admin(frame, page[1], app),
        Screen::Admin => {
            app.login_role = "Administrator".into();
            if app.authenticated {
                draw_admin(frame, page[1], app);
            } else {
                draw_login(frame, page[1], app);
            }
        }
        Screen::District => {
            app.login_role = "District Official".into();
            if app.authenticated {
                draw_district(frame, page[1], app);
            } else {
                draw_login(frame, page[1], app);
            }
        }
        Screen::Vote => {
            app.login_role = "Voter".into();
            if app.authenticated {
                draw_vote(frame, page[1], app);
            } else {
                draw_pin(frame, page[1], app);
            }
        }
    }
    frame.render_widget(
        Paragraph::new(app.notice.as_str())
            .style(Style::new().fg(Color::Yellow))
            .alignment(Alignment::Center),
        page[2],
    );
    if let Some(modal) = &app.modal {
        draw_modal(frame, bounds, modal);
    }
}

fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    let rect = center(area, 68, 55);
    let block = Block::bordered().title_top("Choose a Role").title_bottom(
        Line::from("Arrow keys to navigate - Space to select - Q to quit").centered(),
    );
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let items = [
        "System Administrator",
        "Election Administrator",
        "District Official",
        "Vote!",
    ];
    let list = List::new(
        items
            .iter()
            .map(|item| ListItem::new(Line::from(*item).centered()))
            .collect::<Vec<_>>(),
    )
    .highlight_style(HIGHLIGHT);
    let mut state = ListState::default();
    state.select(Some(app.main_selected));
    let list_height = items.len() as u16;

    let list_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(list_height) / 2,
        width: inner.width,
        height: inner.height.min(list_height),
    };
    frame.render_stateful_widget(list, list_area, &mut state);
}

fn draw_system_admin(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(2),
    ])
    .margin(1)
    .split(area);

    frame.render_widget(
        Paragraph::new("Sign up your administrator account").alignment(Alignment::Center),
        sections[0],
    );

    let email_focused = matches!(app.signup_focus, SignupFocus::Email);
    let password_focused = matches!(app.signup_focus, SignupFocus::Password);

    let email_block = Block::bordered()
        .title_top(if email_focused {
            "Email • typing"
        } else {
            "Email"
        })
        .border_style(Style::new().fg(if email_focused { ACTIVE } else { MUTED }));

    frame.render_widget(
        Paragraph::new(app.login_email.as_str()).block(email_block),
        sections[1],
    );

    // Never display the actual password.
    let masked_password = "*".repeat(app.login_password.chars().count());

    let password_block = Block::bordered()
        .title_top(if password_focused {
            "Password • typing"
        } else {
            "Password"
        })
        .border_style(Style::new().fg(if password_focused { ACTIVE } else { MUTED }));

    frame.render_widget(
        Paragraph::new(masked_password).block(password_block),
        sections[2],
    );

    let phone_number_focused = matches!(app.signup_focus, SignupFocus::PhoneNumber);
    let full_name_focused = matches!(app.signup_focus, SignupFocus::FullName);

    let phone_number_block = Block::bordered()
        .title_top(if phone_number_focused {
            "Phone Number • typing"
        } else {
            "Phone Number"
        })
        .border_style(Style::new().fg(if phone_number_focused { ACTIVE } else { MUTED }));

    frame.render_widget(
        Paragraph::new(app.login_phone_number.as_str()).block(phone_number_block),
        sections[3],
    );

    let full_name_block = Block::bordered()
        .title_top(if full_name_focused {
            "Full Name • typing"
        } else {
            "Full Name"
        })
        .border_style(Style::new().fg(if full_name_focused { ACTIVE } else { MUTED }));

    frame.render_widget(
        Paragraph::new(app.login_full_name.as_str()).block(full_name_block),
        sections[4],
    );

    frame.render_widget(
        Paragraph::new(
            "Tab: switch field | Enter: sign in | Esc: return\n\
            Administrator access requires successful verification.",
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::new().fg(Color::Gray)),
        sections[5],
    );
}

fn draw_admin(frame: &mut Frame, area: Rect, app: &mut App) {
    let outer = Block::bordered()
        .title_bottom(Line::from("Esc to exit - Shift + Enter to Create Election").centered());
    let inside = outer.inner(area);
    frame.render_widget(outer, area);
    let sections =
        Layout::vertical([Constraint::Percentage(54), Constraint::Percentage(46)]).split(inside);
    draw_ballot_editor(frame, sections[0], app);
    let lower = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);
    draw_admin_keys(frame, lower[0], app);
    draw_voters(frame, lower[1], app);
}

fn draw_ballot_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = panel(
        "Create Ballot",
        Line::from("(P) Add Position - (C) Add Candidate - (D) Delete - (E) Edit"),
        app.admin_focus == AdminFocus::Ballot,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let cells = Layout::horizontal([
        Constraint::Percentage(32),
        Constraint::Percentage(42),
        Constraint::Percentage(26),
    ])
    .split(inner);
    let header_style = Style::new()
        .fg(Color::Gray)
        .add_modifier(Modifier::UNDERLINED);
    for (index, heading) in ["Position", "Candidate", "Party"].iter().enumerate() {
        frame.render_widget(
            Paragraph::new(*heading).style(header_style),
            Rect {
                x: cells[index].x,
                y: inner.y,
                width: cells[index].width,
                height: 1,
            },
        );
    }
    if inner.height <= 1 {
        return;
    }
    let height = (inner.height - 1) as usize;
    let rows = app.rows();
    if app.ballot_cursor.row < app.ballot_scroll {
        app.ballot_scroll = app.ballot_cursor.row;
    }
    if app.ballot_cursor.row >= app.ballot_scroll + height {
        app.ballot_scroll = app.ballot_cursor.row + 1 - height;
    }
    for (screen_row, (pi, ci)) in rows.iter().enumerate().skip(app.ballot_scroll).take(height) {
        let y = inner.y + 1 + (screen_row - app.ballot_scroll) as u16;
        let position = &app.positions[*pi];
        let is_first = *ci == Some(0) || ci.is_none();
        let mut texts = [String::new(), String::new(), String::new()];
        if is_first {
            texts[0] = position.name.clone();
        }
        if let Some(candidate_index) = ci {
            let candidate = &position.candidates[*candidate_index];
            texts[1] = candidate.name.clone();
            texts[2] = candidate.party.clone();
        }
        for (col, value) in texts.iter().enumerate() {
            let is_active = app.admin_focus == AdminFocus::Ballot
                && app.ballot_cursor.row == screen_row
                && app.ballot_cursor.column
                    == [
                        BallotColumn::Position,
                        BallotColumn::Candidate,
                        BallotColumn::Party,
                    ][col];
            let style = if is_active {
                HIGHLIGHT
            } else {
                Style::default()
            };
            frame.render_widget(
                Paragraph::new(value.as_str()).style(style),
                Rect {
                    x: cells[col].x,
                    y,
                    width: cells[col].width,
                    height: 1,
                },
            );
        }
    }
}

fn draw_admin_keys(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = panel(
        "Keys",
        Line::from("(I) Import Keys - (U) Unassign Key"),
        app.admin_focus == AdminFocus::Keys,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let items: Vec<ListItem> = app
        .imported_keys
        .iter()
        .map(|key| {
            let assigned = app.voters.iter().find(|v| v.key.as_deref() == Some(key));
            let marker = if assigned.is_some() { "[x]" } else { "[ ]" };
            ListItem::new(format!("{marker} {key}"))
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(if app.admin_focus == AdminFocus::Keys {
                HIGHLIGHT
            } else {
                Style::default()
            })
            .highlight_symbol("> "),
        inner,
        &mut app.admin_key_state,
    );
}

fn draw_voters(frame: &mut Frame, area: Rect, app: &mut App) {
    let help =
        "(O) Open File - (N) New Voter - (R) Register Voter - (A) Register All - (K) Assign Keys";
    let block = panel(
        "Register Voters",
        Line::from(help),
        app.admin_focus == AdminFocus::Voters,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let list_area = help_at_bottom(frame, inner, help);
    let items: Vec<ListItem> = app
        .voters
        .iter()
        .map(|v| {
            ListItem::new(format!(
                "{} {}{}",
                if v.key.is_some() { "[x]" } else { "[ ]" },
                v.full_name, 
                if v.registered { " (registered)" } else { "" }
            ))
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(if app.admin_focus == AdminFocus::Voters {
                HIGHLIGHT
            } else {
                Style::default()
            })
            .highlight_symbol("> "),
        list_area,
        &mut app.voter_state,
    );
}

fn help_at_bottom(frame: &mut Frame, area: Rect, help: &str) -> Rect {
    // A one-cell border title cannot wrap. Also repeat long commands just above
    // the lower border, wrapped across the panel's width, for normal terminals.
    if area.height <= 3 {
        return area;
    }
    let parts = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(area);
    frame.render_widget(
        Paragraph::new(help)
            .wrap(Wrap { trim: true })
            .style(Style::new().fg(Color::Gray)),
        parts[1],
    );
    parts[0]
}

fn draw_district(frame: &mut Frame, area: Rect, app: &mut App) {
    let parts =
        Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)]).split(area);
    let key_block = panel(
        "Generate Keys",
        Line::from("(G) Generate Keys - (C) Clear"),
        app.district_focus == DistrictFocus::Keys,
    );
    let keys_area = key_block.inner(parts[0]);
    frame.render_widget(key_block, parts[0]);
    let key_list = List::new(app.district_keys.iter().map(|k| ListItem::new(k.as_str())))
        .highlight_symbol("> ")
        .highlight_style(if app.district_focus == DistrictFocus::Keys {
            HIGHLIGHT
        } else {
            Style::default()
        });
    frame.render_stateful_widget(key_list, keys_area, &mut app.district_key_state);

    let help = "(E) Enter election - (S) Start Election - (F) Finish Election - (R) Show results";
    let election_block = panel(
        "Run Election",
        Line::from(help),
        app.district_focus == DistrictFocus::Election,
    );
    let content = election_block.inner(parts[1]);
    frame.render_widget(election_block, parts[1]);
    let inner = help_at_bottom(frame, content, help);
    let mut lines = Vec::new();
    if !app.district_entered {
        lines.push(Line::from("Enter an election hash using E."));
    } else if let Some(e) = &app.election {
        lines.push(
            Line::from(format!("Status: {}", e.phase.label()))
                .style(Style::new().fg(Color::Yellow)),
        );
        lines.push(Line::from(""));
        for p in &e.positions {
            lines.push(Line::from(p.name.clone()).style(Style::new().add_modifier(Modifier::BOLD)));
            for c in &p.candidates {
                let detail = if e.phase == Phase::Finished {
                    format!("  {}  |  {}  |  {} votes", c.name, c.party, c.votes)
                } else {
                    format!("  {}  |  {}", c.name, c.party)
                };
                lines.push(Line::from(detail));
            }
            lines.push(Line::from(""));
        }
    }
    let max_scroll = lines.len().saturating_sub(inner.height as usize);
    app.district_scroll = app.district_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.district_scroll as u16, 0)),
        inner,
    );
}

fn draw_vote(frame: &mut Frame, area: Rect, app: &mut App) {
    let phase = app.election.as_ref().map(|e| e.phase);
    let ready = app.voting_key.is_some() && phase == Some(Phase::Running);
    let footer = Line::from(vec![
        Span::raw("(E) Enter Election - "),
        Span::styled(
            "(S) Submit Ballot",
            if ready {
                Style::new().fg(Color::White)
            } else {
                Style::new().fg(MUTED)
            },
        ),
        Span::raw(" - (V) View Results"),
        Span::raw(if app.voting_key.is_some() {
            " - (Space) to Select"
        } else {
            ""
        }),
    ]);
    let rect = center(area, 92, 94);
    let block = Block::bordered().title_top("Vote!").title_bottom(footer);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    if inner.height == 0 {
        return;
    }
    let Some(e) = &app.election else {
        frame.render_widget(
            Paragraph::new("No election created. Ask an Election Administrator to create one."),
            inner,
        );
        return;
    };
    if app.voting_key.is_none() && e.phase != Phase::Finished {
        frame.render_widget(
            Paragraph::new(format!(
                "Status: {}\nPress E to enter an assigned voting key.",
                e.phase.label()
            )),
            inner,
        );
        return;
    }
    let rows = ballot_rows(&e.positions);
    let status =
        Line::from(format!("Status: {}", e.phase.label())).style(Style::new().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new(status),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );
    if inner.height <= 1 {
        return;
    }
    let height = (inner.height - 1) as usize;
    app.voting_row = app.voting_row.min(rows.len().saturating_sub(1));
    if app.voting_row < app.voting_scroll {
        app.voting_scroll = app.voting_row;
    }
    if app.voting_row >= app.voting_scroll + height {
        app.voting_scroll = app.voting_row + 1 - height;
    }
    for (row_idx, (pi, ci)) in rows.iter().enumerate().skip(app.voting_scroll).take(height) {
        let y = inner.y + 1 + (row_idx - app.voting_scroll) as u16;
        let p = &e.positions[*pi];
        let Some(ci) = ci else {
            continue;
        };
        let c = &p.candidates[*ci];
        let mark = if e.phase == Phase::Finished {
            format!("{} votes", c.votes)
        } else if app.voting_choices.get(*pi) == Some(&Some(*ci)) {
            "[x]".into()
        } else {
            "[ ]".into()
        };
        let label = if *ci == 0 { &p.name } else { "" };
        let selected =
            row_idx == app.voting_row && e.phase == Phase::Running && app.voting_key.is_some();
        let style = if selected {
            HIGHLIGHT
        } else {
            Style::default()
        };
        let cols = Layout::horizontal([
            Constraint::Percentage(27),
            Constraint::Percentage(36),
            Constraint::Percentage(24),
            Constraint::Percentage(13),
        ])
        .split(Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        });
        for (i, value) in [label, c.name.as_str(), c.party.as_str(), mark.as_str()]
            .iter()
            .enumerate()
        {
            frame.render_widget(Paragraph::new(*value).style(style), cols[i]);
        }
    }
}

fn draw_modal(frame: &mut Frame, area: Rect, modal: &Modal) {
    let rect = center(area, 66, 30);
    frame.render_widget(Clear, rect);
    let (title, footer): (Line<'static>, Line<'static>) = match modal {
        Modal::Input(d) => (
            Line::from(d.title.clone()).centered(),
            Line::from("Esc to cancel - Enter to write"),
        ),
        Modal::Confirm { title, .. } => (
            Line::from(title.clone()).centered(),
            Line::from("Enter/Y to confirm - Esc/N to cancel"),
        ),
        Modal::Info {
            title, copy_hash, ..
        } => (
            Line::from(title.clone()).centered(),
            if *copy_hash {
                Line::from("Esc to exit to Main Menu - (C) to copy to clipboard")
            } else {
                Line::from("Enter / Esc to close")
            },
        ),
    };
    let block = Block::bordered()
        .border_style(Style::new().fg(ACTIVE))
        .title_top(title)
        .title_bottom(footer);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let body = match modal {
        Modal::Input(d) => {
            let mut text = format!(
                "{}\n\n{}▏",
                match d.purpose {
                    InputPurpose::Edit(_) => "Change the selected field:",
                    InputPurpose::ImportVoters => "Path to a text/CSV file (one contact per line):",
                    InputPurpose::NewVoter => "Email address or phone number:",
                    InputPurpose::GenerateKeys => "How many keys? (1-10000)",
                    InputPurpose::EnterElection => "Paste the election hash:",
                    InputPurpose::EnterVotingKey => "Enter your assigned key:",
                },
                d.text
            );
            if let Some(error) = &d.error {
                text.push_str(&format!("\n\nError: {error}"));
            }
            text
        }
        Modal::Confirm { message, .. } | Modal::Info { message, .. } => message.clone(),
    };
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), inner);
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut app = App::default();
    app.conn =
        Some(auth::login::initialize_database().map_err(|err| io::Error::other(err.to_string()))?);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;
        if app.quit {
            break;
        }
        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_are_scoped_to_their_positions() {
        let mut app = App::default();
        app.ballot_cursor.row = 2; // first candidate in Position 1
        app.add_candidate();
        assert_eq!(app.positions[0].candidates.len(), 2);
        assert_eq!(app.positions[1].candidates.len(), 3);
        assert_eq!(app.positions[1].candidates[2].name, "Candidate c");
    }

    #[test]
    fn allocation_is_unique_and_transactional() {
        let mut app = App::default();

        app.voters = vec![
            Voter {
                full_name: "First Voter".into(),
                email: "a@example.com".into(),
                phone_number: "1234567890".into(),
                registered: true,
                key: None,
            },
            Voter {
                full_name: "Second Voter".into(),
                email: "b@example.com".into(),
                phone_number: "0987654321".into(),
                registered: true,
                key: None,
            },
        ];

        // Insufficient keys: assignment must fail without partial changes.
        app.imported_keys = vec!["key-1".into()];

        assert!(app.assign_keys().is_err());
        assert!(app.voters.iter().all(|v| v.key.is_none()));

        // Enough keys: each voter receives a different key.
        app.imported_keys.push("key-2".into());

        app.assign_keys().expect("Key assignment should succeed");

        assert_ne!(app.voters[0].key, app.voters[1].key);
        assert!(app.voters.iter().all(|v| v.key.is_some()));
    }

    #[test]
    fn ballot_cannot_be_submitted_twice() {
        let mut app = App::default();

        // One voter is enough to test duplicate ballot submission.
        app.voters = vec![Voter {
            full_name: "Test Voter".into(),
            email: "a@example.com".into(),
            phone_number: "1234567890".into(),
            registered: true,
            key: None,
        }];

        app.imported_keys.push("test-key".into());

        app.assign_keys().expect("Key assignment should succeed");
        app.create_election().expect("Election creation should succeed");

        let election = app.election.as_mut().expect("Election should exist");

        start_election(election).expect("Election should open");

        let choices = [Some(0), Some(1), Some(0)];

        assert!(submit_ballot(election, "test-key", &choices).is_ok());
        assert!(submit_ballot(election, "test-key", &choices).is_err());

        stop_election(election).expect("Election should close");

        assert_eq!(election.positions[0].candidates[0].votes, 1);
    }
}

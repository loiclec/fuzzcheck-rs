use termion::event::Key;
use tui::{Frame, backend::Backend, layout::Rect, style::{Color, Style}, text::{Span, Spans}, widgets::{Borders, Paragraph, Wrap}};

pub trait AnyView {
    fn focus(&mut self);
    fn unfocus(&mut self);
    fn key_bindings(&self) -> Vec<(Key, String)>;
}

pub trait ViewState: AnyView {
    type Update;
    type InMessage;
    type OutMessage;

    fn convert_in_message(&self, message: Self::InMessage) -> Option<Self::Update>;
    fn update(&mut self, u: Self::Update) -> Option<Self::OutMessage>;
    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, chunk: Rect)
    where
        B: Backend;
}

pub trait ParentView<Child: ViewState>: ViewState {
    fn convert_child_update(update: Child::Update) -> Self::Update;

    fn convert_to_child_in_message(message: &Self::InMessage) -> Option<Child::InMessage>;

    fn handle_child_in_message(child: &Child, message: &Self::InMessage) -> Option<Self::Update> {
        Self::convert_to_child_in_message(message)
            .and_then(|message| child.convert_in_message(message))
            .map(|child_update| Self::convert_child_update(child_update))
    }

    fn convert_child_out_message(&self, message: Child::OutMessage) -> Either<Self::Update, Self::OutMessage>;

    fn handle_child_out_message(&mut self, message: Child::OutMessage) -> Option<Self::OutMessage> {
        match self.convert_child_out_message(message) {
            Either::Left(u) => self.update(u),
            Either::Right(out) => Some(out),
        }
    }
}

pub trait InnerFocusable {
    type Focus: Copy;

    fn focus(&mut self) -> &mut Self::Focus;

    fn focus_after_switch(&self, sf: SwitchFocus) -> Option<Self::Focus>;

    fn view_in_focus_ref(&self) -> Option<&dyn AnyView>;
    fn view_in_focus_mut(&mut self) -> Option<&mut dyn AnyView>;

    fn update_focus(&mut self, focus: Self::Focus) {
        if let Some(v) = self.view_in_focus_mut() {
            v.unfocus()
        };
        *self.focus() = focus;
        if let Some(v) = self.view_in_focus_mut() {
            v.focus()
        };
    }
}

pub struct ExplainKeyBindingView {
    explanations: Vec<(Key, String)>
}
impl ExplainKeyBindingView {
    pub fn new(explanations: Vec<(Key, String)>) -> Self {
        Self {
            explanations
        }
    }
}

pub fn override_map(map: &mut Vec<(Key, String)>, merging: Vec<(Key, String)>) {
    for (key, value) in merging {
        if let Some(key_idx) = map.iter().position(|x| x.0 == key) {
            map.remove(key_idx);
        }
        map.push((key, value));
    }
}

impl AnyView for ExplainKeyBindingView {
    fn focus(&mut self) {
    }

    fn unfocus(&mut self) {
    }

    fn key_bindings(&self) -> Vec<(Key, String)> {
        Vec::new()
    }
}

impl ViewState for ExplainKeyBindingView {
    type Update = ();
    type InMessage = ();
    type OutMessage = ();

    fn convert_in_message(&self, _message: Self::InMessage) -> Option<Self::Update> {
        None
    }

    fn update(&mut self, _u: Self::Update) -> Option<Self::OutMessage> {
        None
    }

    fn draw<B>(&self, frame: &mut Frame<B>, theme: &Theme, chunk: Rect) where B: Backend {
        let mut text = Vec::<Span>::new();
        for (key, explanation) in self.explanations.iter() {
            text.push(Span::styled(format!("{}:", display_key(key)), theme.emphasis));
            text.push(Span::from(format!(" {} ", explanation)));
        }
        let p = Paragraph::new(Spans::from(text)).wrap(Wrap {trim: true }).style(theme.default);
        frame.render_widget(p, chunk);
    }
}
fn display_key(key: &Key) -> String {
    match key {
        Key::Backspace => { "backspace".to_string() }
        Key::Left => { "←".to_string() }
        Key::Right => { "→".to_string() }
        Key::Up => { "↑".to_string() }
        Key::Down => { "↓".to_string() }
        Key::Home => { "home".to_string() }
        Key::End => { "end".to_string() }
        Key::PageUp => { "page up".to_string() }
        Key::PageDown => { "page down".to_string() }
        Key::BackTab => { "backtab".to_string() }
        Key::Delete => { "del".to_string() }
        Key::Insert => { "insert".to_string() }
        Key::F(x) => { format!("f{}", x) }
        Key::Char('\t') => { "tab".to_string() }
        Key::Char('\n') => { "enter".to_string() }
        Key::Char(x) => { format!("{}", x) }
        Key::Alt(x) => { format!("alt+{}", x) }
        Key::Ctrl(x) => { format!("ctrl+{}", x) }
        Key::Null => { "null".to_string() }
        Key::Esc => { "esc".to_string() }
        Key::__IsNotComplete => { "".to_string() }
    }
}

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

// pub enum Move {
//     Vertical(VerticalMove),
//     Horizontal(HorizontalMove),
// }
pub enum VerticalMove {
    Up,
    Down,
}
pub enum HorizontalMove {
    Left,
    Right,
}

pub enum SwitchFocus {
    In, Out, Up, Right, Down, Left, Next, Previous
}
impl SwitchFocus {
    pub fn from_standard_key(key: &Key) -> Option<SwitchFocus> {
        match key {
            Key::Left => { Some(SwitchFocus::Left) }
            Key::Right => {Some(SwitchFocus::Right)}
            Key::Up => {Some(SwitchFocus::Up)}
            Key::Down => {Some(SwitchFocus::Down)}
            Key::BackTab => {Some(SwitchFocus::Previous)}
            Key::Char('\t') => {Some(SwitchFocus::Next)}
            Key::Char('\n') => { Some(SwitchFocus::In) }
            Key::Esc => { Some(SwitchFocus::Out) }
            _ => { None }
        }
    }
}

impl VerticalMove {
    pub fn from(input: &Key) -> Option<Self> {
        match input {
            Key::Up => Some(Self::Up),
            Key::Down => Some(Self::Down),
            _ => None,
        }
    }
}
impl HorizontalMove {
    pub fn from(input: &Key) -> Option<Self> {
        match input {
            Key::Left => Some(Self::Left),
            Key::Right => Some(Self::Right),
            _ => None,
        }
    }
}
// impl Move {
//     pub fn from(input: &Key) -> Option<Self> {
//         HorizontalMove::from(input).map(Move::Horizontal)
//         .or(VerticalMove::from(input).map(Move::Vertical))
//     }
// }

pub struct Theme {
    pub focus: bool,
    pub block_borders: Borders,
    pub default: Style,
    pub highlight: Style,
    pub block_highlight: Style,
    pub emphasis: Style,
    pub error: Style,
    pub disabled: Style,
}

impl Theme {
    pub fn primary() -> Self {
        Self {
            focus: true,
            block_borders: Borders::ALL,
            default: Style::default().bg(Color::Black).fg(Color::White),
            highlight: Style::default().bg(Color::Yellow).fg(Color::Black),
            block_highlight: Style::default().bg(Color::Black).fg(Color::Yellow),
            emphasis: Style::default().bg(Color::Black).fg(Color::LightBlue),
            error: Style::default().bg(Color::Black).fg(Color::Red),
            disabled: Style::default().bg(Color::Black).fg(Color::DarkGray),
        }
    }
    pub fn secondary() -> Self {
        Self {
            focus: false,
            block_borders: Borders::ALL,
            default: Style::default().bg(Color::Black).fg(Color::DarkGray),
            highlight: Style::default().bg(Color::DarkGray).fg(Color::Gray),
            block_highlight: Style::default().bg(Color::Black).fg(Color::White),
            emphasis: Style::default().bg(Color::Black).fg(Color::DarkGray),
            error: Style::default().bg(Color::Black).fg(Color::Gray),
            disabled: Style::default().bg(Color::Black).fg(Color::DarkGray),
        }
    }
}

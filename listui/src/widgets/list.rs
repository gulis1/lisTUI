use listui_lib::models::Drawable;
use ratatui::style::{Style, Color, Modifier};
use ratatui::text::Span;
use ratatui::widgets::{ListState, ListItem, List};
use ratatui::Frame;
use ratatui::layout::Rect;
use rand::seq::SliceRandom;
use rand::thread_rng;

// Generic list widget, that support drawing a filtered view of itself.
// The filtering is only computed when the search query changes.
pub struct ListWidget<T: Drawable> {

    title: String,
    state: ListState,
    items: Vec<T>,
    
    shuffled: bool,
    ordered_items: Vec<usize>,
    last_query: Option<String>,
    filtered_indexes: Vec<usize>,
    filter_state: ListState
}

impl<T: Drawable> ListWidget<T> {

    pub fn empty(title: &str) -> Self {

        Self {
            title: String::from(title),
            state: ListState::default(),
            items: Vec::new(),
            shuffled: false,
            ordered_items: Vec::new(),
            last_query: None,
            filtered_indexes: Vec::new(),
            filter_state: ListState::default(),
        }
    }
    
    pub fn with_items(title: &str, items: Vec<T>) -> Self {

        Self {
            title: String::from(title),
            state: ListState::default(),
            
            ordered_items: (0..items.len()).collect(),
            shuffled: false,
            items,
            last_query: None,
            filtered_indexes: Vec::new(),
            filter_state: ListState::default(),
        }
    }

    pub fn get_selected(&self) -> Option<usize> {

        if self.is_filtered() {
            self.filter_state.selected().map(|ind| self.filtered_indexes[ind])
        }
        else { self.state.selected() }
    }

    pub fn next(&mut self) {
        
        let filtered = self.is_filtered();
        let st = if filtered { &mut self.filter_state } else { &mut self.state };
        let len = if filtered { self.filtered_indexes.len() } else { self.items.len() };
        
        if len > 0 {

            let next = match st.selected() {
                Some(i) => { (i + 1) % len },
                None => 0,
            };
            st.select(Some(next));
        }   
    }

    pub fn previous(&mut self) {
        
        let filtered = self.is_filtered();
        let st = if filtered { &mut self.filter_state } else { &mut self.state };
        let len = if filtered { self.filtered_indexes.len() } else { self.items.len()} ;

        if len > 0 {

            let i = match st.selected() {
                Some(i) => {
                    if i == 0 { len - 1 } 
                    else { i - 1 }
                }
                None => 0,
            };
            st.select(Some(i));
        }
    }

    pub fn select_ind(&mut self, ind: usize) {

        let filtered = self.is_filtered();
        let st = if filtered { &mut self.filter_state } else { &mut self.state };
        let len = if filtered { self.filtered_indexes.len() } else { self.items.len() };

        if ind < len {
            st.select(Some(ind));
        }
    }

    pub fn filter(&mut self, query: &str) {

        // self.last_query cannot be none is self.filtered is true
        // so using unwrap shuold be safe here.
        //
        
        let query = query.to_lowercase();
        if !self.is_filtered() || self.last_query.as_ref().unwrap() != &query {

            self.filtered_indexes = self.ordered_items.iter()
                .enumerate()
                .filter(|(_, i)| self.items[**i].get_text().to_ascii_lowercase().contains(&query))
                .map(|(ind, _)| ind)
                .collect();

            self.filter_state = ListState::default();
            self.last_query = Some(String::from(query));
        }    
    }

    pub fn clear_filter(&mut self) {
        self.last_query = None;
    }

    pub fn is_filtered(&self) -> bool {
        self.last_query.is_some()
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        
        if self.is_filtered() { self.draw_filtered(frame, area); }
        else { self.draw_all(frame, area); }
    }

    fn draw_all(&mut self, frame: &mut Frame, area: Rect) {

        let items: Vec<ListItem> = self.ordered_items
            .iter()
            .map(|i| {
                let lines = Span::from(self.items[*i].get_text());
                ListItem::new(lines).style(Style::default())
            })
            .collect();
        
        let list = List::new(items)
            .block(super::BLOCK.clone().title(Span::styled(self.title.as_str(), Style::default().add_modifier(Modifier::BOLD))))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(super::ACC_COLOR)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }

    fn draw_filtered(&mut self, frame: &mut Frame, area: Rect)  {

        let filtered: Vec<ListItem> = self.filtered_indexes
            .iter()
            .map(|ind| {

                let lines = Span::from(self.items[self.ordered_items[*ind]].get_text());
                ListItem::new(lines).style(Style::default())
            })
            .collect();
        
        let title = format!(" ≫  Search: {} ", self.last_query.as_ref().expect("No query to search."));
        let list = List::new(filtered)
            .block(super::BLOCK.clone().title(title.as_str()))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(super::ACC_COLOR)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.filter_state);
    }

    pub fn get_ind(&self, ind: usize) -> &T {
        &self.items[self.ordered_items[ind]]
    }

    pub fn total_len(&self) -> usize {
        self.items.len()
    }

    pub fn toggle_shuffle(&mut self) {
        

        if self.shuffled {
            self.ordered_items = (0..self.items.len()).collect();
            self.state = ListState::default();
            self.shuffled = false;
            self.title.pop();
            self.title.pop();
            self.title.pop();
            self.title.pop();
        }
        else {
            let mut rng = thread_rng();
            self.ordered_items.shuffle(&mut rng);
            self.state = ListState::default();
            self.shuffled = true;
            self.title.push_str(" ⤨  ");
        }   
    }
}

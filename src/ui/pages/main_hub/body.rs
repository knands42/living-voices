use crate::backend::domain::adventure::{AdventureStatus, AdventureSummary, OngoingAdventure};
use crate::ui::pages::main_hub::{Message, State};
use crate::ui::{common, theme};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};

const RAIL_LENGTH: f32 = 320.0;
const PLATE_LENGTH: f32 = 292.0;
const SHELF_CARD_WIDTH: f32 = 280.0;
const SHELF_CARD_HEIGHT: f32 = 92.0;
const GRID_CARD_HEIGHT: f32 = 196.0;
const GRID_CARD_MIN_WIDTH: f32 = 170.0;
const GRID_SPACING: f32 = 12.0;
const PAGE_PADDING: f32 = 44.0;
const MAX_COLUMNS: usize = 8;
const MIN_COLUMNS: usize = 2;

pub fn body(home_state: &State) -> Element<'_, Message> {
    if home_state.catalogue.adventures.is_empty() {
        return empty_state();
    }

    let main = column![
        featured(home_state),
        shelf(home_state),
        catalogue(home_state)
    ]
    .spacing(18.0)
    .width(Length::Fill);

    let rail = column![create_card(), stats(home_state)]
        .spacing(14.0)
        .width(Length::Fixed(RAIL_LENGTH));

    row![
        scrollable(main).height(Length::Fill).width(Length::Fill),
        container(rail)
            .width(Length::Fixed(RAIL_LENGTH))
            .height(Length::Fill)
            .align_y(Alignment::Start),
    ]
    .spacing(14.0)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// --- the story of the week ------------------------------------------------

fn featured(home_state: &State) -> Element<'_, Message> {
    let Some(newest) = home_state
        .catalogue
        .adventures
        .iter()
        .max_by_key(|adventure| adventure.created_at)
    else {
        return empty_state();
    };

    let hue = hue_of(&newest.title);
    let accent = theme::hsv(hue, 0.45, 0.82);

    let plate = container(
        column![
            text("FEATURED")
                .size(9.5)
                .color(theme::alpha(theme::TEXT_LIGHT, 0.55))
                .font(common::semibold()),
            Space::new().height(Length::Fill),
            common::initials(monogram(&newest.title), hue, 56.0, 0.7),
            Space::new().height(Length::Fill),
            text(newest.status.as_str().to_uppercase())
                .size(10.5)
                .color(theme::alpha(theme::TEXT_LIGHT, 0.62)),
        ]
        .spacing(10.0)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fixed(PLATE_LENGTH))
    .height(Length::Fixed(232.0))
    .padding(16.0)
    .style(theme::cover(hue));

    let facts = row![
        text(newest.status.as_str().to_uppercase())
            .size(9.5)
            .color(accent)
            .font(common::semibold()),
        common::dot(4.0, theme::alpha(accent, 0.7)),
        text(newest.updated_at.format("%b %d, %Y").to_string())
            .size(10.5)
            .color(theme::TEXT_DIM),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center);

    let body = column![
        facts,
        text(newest.title.as_str())
            .size(32.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(newest.headline.as_str())
            .size(14.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        common::divider(),
        text(newest.tone.as_str())
            .size(12.5)
            .color(theme::alpha(theme::TEXT_MUTED, 0.92))
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        Space::new().height(Length::Fill),
        row![
            button(text("Begin the story").size(13.0))
                .padding([11.0, 20.0])
                .style(theme::send_button)
                .on_press(Message::ChatRequested),
            Space::new().width(Length::Fill),
        ]
        .spacing(12.0)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    ]
    .spacing(9.0)
    .width(Length::Fill)
    .height(Length::Fill);

    common::panel(
        row![
            plate,
            container(body).width(Length::Fill).padding([2.0, 0.0]),
        ]
        .spacing(20.0)
        .width(Length::Fill),
        16.0,
    )
    .into()
}

// --- what you already have open -------------------------------------------

fn shelf(home_state: &State) -> Element<'_, Message> {
    if home_state.catalogue.shelf.len() == 0 {
        return column![].spacing(10.0).width(Length::Fill).into();
    }

    let header = row![
        common::heading("Pick up where you left off"),
        Space::new().width(Length::Fill),
        text(format!("{} open", home_state.catalogue.shelf.len()))
            .size(10.5)
            .color(theme::TEXT_DIM),
    ]
    .align_y(Alignment::Center);

    let mut cards = row![].spacing(12.0);
    for entry in &home_state.catalogue.shelf {
        cards = cards.push(shelf_card(entry));
    }

    let strip = scrollable(container(cards).padding(iced::Padding::ZERO.bottom(10.0)))
        .horizontal()
        .width(Length::Fill);

    column![header, strip]
        .spacing(10.0)
        .width(Length::Fill)
        .into()
}

fn shelf_card(entry: &OngoingAdventure) -> Element<'_, Message> {
    let adventure = &entry.adventure;
    let hue = hue_of(&adventure.title);
    let accent = theme::hsv(hue, 0.42, 0.80);
    let progress = if entry.total_acts > 0 {
        entry.current_act as f32 / entry.total_acts as f32
    } else {
        0.0
    };

    let cover = container(
        text(monogram(&adventure.title))
            .size(18.0)
            .color(theme::alpha(theme::TEXT_LIGHT, 0.85))
            .font(common::brand()),
    )
    .width(Length::Fixed(44.0))
    .height(Length::Fixed(44.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(theme::cover(hue));

    let head = row![
        cover,
        column![
            text(adventure.title.as_str())
                .size(13.5)
                .color(theme::TEXT_LIGHT)
                .font(common::semibold())
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text(format!(
                "act {} of {} · {}",
                entry.current_act,
                entry.total_acts,
                entry.last_played.format("%b %d").to_string()
            ))
            .size(10.5)
            .color(theme::TEXT_DIM),
        ]
        .spacing(3.0)
        .width(Length::Fill),
    ]
    .spacing(10.0)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let foot = row![
        common::meter(
            progress,
            theme::alpha(accent, 0.95),
            theme::alpha(theme::BRONZE_DIM, 0.55),
            3.0,
        ),
        text(format!("{:.0}%", progress * 100.0))
            .size(10.0)
            .color(theme::TEXT_DIM),
    ]
    .spacing(9.0)
    .align_y(Alignment::Center);

    mouse_area(
        container(column![head, foot].spacing(11.0).width(Length::Fill))
            .padding(12.0)
            .width(Length::Fixed(SHELF_CARD_WIDTH))
            .height(Length::Fixed(SHELF_CARD_HEIGHT))
            .style(theme::shelf_card(hue, 0.0)),
    )
    .on_press(Message::OpenAdventure(adventure.id))
    .into()
}

// --- the catalogue --------------------------------------------------------

fn catalogue(home_state: &State) -> Element<'_, Message> {
    let query = home_state.query.trim().to_lowercase();
    let tags = collect_tags(home_state);

    let filtered: Vec<&AdventureSummary> = home_state
        .catalogue
        .adventures
        .iter()
        .filter(|adventure| matches_query(adventure, &query))
        .filter(|adventure| matches_tag(adventure, home_state.active_tag.as_deref()))
        .collect();

    let header = row![
        common::heading("The library"),
        text(format!(
            "{} of {}",
            filtered.len(),
            home_state.catalogue.adventures.len()
        ))
        .size(10.5)
        .color(theme::TEXT_DIM),
        Space::new().width(Length::Fill),
    ]
    .spacing(10.0)
    .align_y(Alignment::Center);

    let list = if filtered.is_empty() {
        no_matches()
    } else {
        grid(&filtered, columns_for(home_state.width))
    };

    if tags.is_empty() {
        return column![header, list]
            .spacing(12.0)
            .width(Length::Fill)
            .into();
    }

    column![
        header,
        filter_bar(home_state.active_tag.as_deref(), &tags),
        list
    ]
    .spacing(12.0)
    .width(Length::Fill)
    .into()
}

fn columns_for(window_width: f32) -> usize {
    let area = window_width - PAGE_PADDING - RAIL_LENGTH - 14.0;
    let fitting = ((area + GRID_SPACING) / (GRID_CARD_MIN_WIDTH + GRID_SPACING)) as usize;

    fitting.clamp(MIN_COLUMNS, MAX_COLUMNS)
}

fn grid<'a>(adventures: &[&'a AdventureSummary], columns: usize) -> Element<'a, Message> {
    let mut rows = column![].spacing(GRID_SPACING).width(Length::Fill);

    for chunk in adventures.chunks(columns) {
        let mut line = row![].spacing(GRID_SPACING).width(Length::Fill);

        for adventure in chunk {
            line = line.push(grid_card(adventure));
        }
        for _ in chunk.len()..columns {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }

        rows = rows.push(line);
    }

    rows.into()
}

fn grid_card(adventure: &AdventureSummary) -> Element<'_, Message> {
    let hue = hue_of(&adventure.title);

    let cover = container(
        text(monogram(&adventure.title))
            .size(20.0)
            .color(theme::alpha(theme::TEXT_LIGHT, 0.85))
            .font(common::brand()),
    )
    .width(Length::Fixed(44.0))
    .height(Length::Fixed(44.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(theme::cover(hue));

    let top = row![
        cover,
        Space::new().width(Length::Fill),
        status_badge(adventure.status)
    ]
    .align_y(Alignment::Start)
    .width(Length::Fill);

    let body = column![
        top,
        text(adventure.title.as_str())
            .size(14.0)
            .color(theme::TEXT_LIGHT)
            .font(common::semibold())
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        text(adventure.headline.as_str())
            .size(11.5)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        Space::new().height(Length::Fill),
        text(meta_line(adventure))
            .size(9.5)
            .color(theme::TEXT_DIM)
            .width(Length::Fill),
    ]
    .spacing(6.0)
    .width(Length::Fill)
    .height(Length::Fill);

    mouse_area(
        container(body)
            .padding(12.0)
            .width(Length::FillPortion(1))
            .height(Length::Fixed(GRID_CARD_HEIGHT))
            .clip(true)
            .style(theme::shelf_card(hue, 0.0)),
    )
    .on_press(Message::OpenAdventure(adventure.id))
    .into()
}

fn no_matches() -> Element<'static, Message> {
    container(
        column![
            text("Nothing matches").size(13.5).color(theme::TEXT_LIGHT),
            text("Try a different word.")
                .size(11.5)
                .color(theme::TEXT_DIM),
        ]
        .spacing(8.0)
        .align_x(Alignment::Center),
    )
    .padding(26.0)
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .style(theme::inset)
    .into()
}

// --- the rail -------------------------------------------------------------

fn create_card() -> Element<'static, Message> {
    container(
        column![
            common::eyebrow("your own story"),
            text("Nothing here is written yet")
                .size(19.0)
                .color(theme::GOLD)
                .font(common::brand())
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            button(text("Build an adventure").size(13.0))
                .padding([10.0, 16.0])
                .width(Length::Fill)
                .style(theme::send_button)
                .on_press(Message::AdventureDraftRequested),
        ]
        .spacing(11.0)
        .width(Length::Fill),
    )
    .padding(16.0)
    .width(Length::Fixed(RAIL_LENGTH))
    .style(theme::create_card)
    .into()
}

fn stats(home_state: &State) -> Element<'_, Message> {
    let drafts = home_state
        .catalogue
        .adventures
        .iter()
        .filter(|adventure| adventure.status == AdventureStatus::Draft)
        .count();

    column![
        common::eyebrow("your shelf"),
        row![
            stat_cell("stories", home_state.catalogue.adventures.len().to_string()),
            stat_cell("drafts", drafts.to_string()),
            stat_cell("open", home_state.catalogue.shelf.len().to_string()),
        ]
        .spacing(7.0)
        .width(Length::Fill),
    ]
    .spacing(8.0)
    .width(Length::Fill)
    .into()
}

fn stat_cell(label: &'static str, value: String) -> Element<'static, Message> {
    container(
        column![
            common::eyebrow(label),
            text(value)
                .size(14.0)
                .color(theme::TEXT_LIGHT)
                .font(common::semibold()),
        ]
        .spacing(3.0),
    )
    .padding([9.0, 11.0])
    .width(Length::Fill)
    .style(theme::stat)
    .into()
}

fn empty_state() -> Element<'static, Message> {
    let panel = common::panel(
        column![
            text("Nothing here is written yet")
                .size(24.0)
                .color(theme::GOLD)
                .font(common::brand()),
            text("Build your first adventure and the library will grow around it.")
                .size(13.0)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            button(text("Build an adventure").size(13.0))
                .padding([11.0, 20.0])
                .style(theme::send_button)
                .on_press(Message::AdventureDraftRequested),
        ]
        .spacing(12.0)
        .width(Length::Fill)
        .align_x(Alignment::Center),
        28.0,
    );

    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}

// --- the create adventure pop-up ------------------------------------------

pub fn new_adventure_popup(home_state: &State) -> Element<'_, Message> {
    let draft = &home_state.new_adventure_draft;

    let card = container(
        column![
            common::eyebrow("a new story"),
            text("Begin your own")
                .size(26.0)
                .color(theme::GOLD)
                .font(common::brand()),
            text("Title")
                .size(12.0)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text_input("The Falling Stars", &draft.title)
                .on_input(Message::AdventureDraftTitleChanged)
                .on_submit(Message::AdventureDraftSubmitted)
                .padding([11.0, 13.0])
                .size(13.0)
                .style(theme::composer),
            text("Headline - one true sentence about the world is enough to start")
                .size(12.0)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text_input(
                "A harbour town where the tide keeps the debts",
                &draft.headline
            )
            .on_input(Message::AdventureDraftHeadlineChanged)
            .on_submit(Message::AdventureDraftSubmitted)
            .padding([11.0, 13.0])
            .size(13.0)
            .style(theme::composer),
            row![
                button(text("Cancel").size(12.5))
                    .padding([9.0, 16.0])
                    .style(theme::back_button)
                    .on_press(Message::AdventureDraftCancelled),
                Space::new().width(Length::Fill),
                button(text("Begin").size(13.0))
                    .padding([10.0, 22.0])
                    .style(theme::send_button)
                    .on_press(Message::AdventureDraftSubmitted),
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center),
        ]
        .spacing(12.0)
        .width(Length::Fill),
    )
    .padding(24.0)
    .width(Length::Fixed(540.0))
    .style(theme::lightbox_card);

    let card = mouse_area(card).on_press(Message::Noop);

    mouse_area(
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(theme::lightbox),
    )
    .on_press(Message::AdventureDraftCancelled)
    .into()
}

// --- small things ---------------------------------------------------------

fn status_badge(status: AdventureStatus) -> Element<'static, Message> {
    let (label, color) = match status {
        AdventureStatus::Draft => ("draft", theme::TEXT_DIM),
        AdventureStatus::Private => ("private", theme::SEA),
        AdventureStatus::Public => ("public", theme::AMBER),
    };
    badge(label, color)
}

fn badge(label: &'static str, color: iced::Color) -> Element<'static, Message> {
    container(text(label).size(9.0).color(color).font(common::semibold()))
        .padding([2.0, 7.0])
        .style(theme::badge(color))
        .into()
}

fn monogram(title: &str) -> &str {
    title.get(..1).unwrap_or("·")
}

fn filter_bar(active_tag: Option<&str>, tags: &[String]) -> Element<'static, Message> {
    let mut chips = row![].spacing(7.0);
    chips = chips.push(tag_chip(None, "All".to_string(), active_tag.is_none()));
    for tag in tags {
        let selected = active_tag == Some(tag.as_str());
        chips = chips.push(tag_chip(Some(tag.as_str()), tag.clone(), selected));
    }
    chips.into()
}

fn tag_chip(tag: Option<&str>, label: String, selected: bool) -> Element<'static, Message> {
    button(text(label).size(12.0))
        .padding([7.0, 13.0])
        .style(theme::chip(selected))
        .on_press(Message::TagSelected(tag.map(str::to_owned)))
        .into()
}

fn collect_tags(home_state: &State) -> Vec<String> {
    let mut tags: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for adventure in &home_state.catalogue.adventures {
        tags.extend(adventure.tags.iter().cloned());
    }
    tags.into_iter().collect()
}

fn matches_query(adventure: &AdventureSummary, query: &str) -> bool {
    query.is_empty()
        || adventure.title.to_lowercase().contains(query)
        || adventure.headline.to_lowercase().contains(query)
}

fn matches_tag(adventure: &AdventureSummary, active_tag: Option<&str>) -> bool {
    match active_tag {
        None => true,
        Some(tag) => adventure.tags.iter().any(|candidate| candidate == tag),
    }
}

fn meta_line(adventure: &AdventureSummary) -> String {
    let updated = adventure.updated_at.format("%b %d, %Y").to_string();
    if adventure.tags.is_empty() {
        format!("updated {updated}")
    } else {
        format!("{} · updated {updated}", adventure.tags.join(" · "))
    }
}

fn hue_of(title: &str) -> f32 {
    let mut sum: u32 = 0;
    for byte in title.bytes() {
        sum = sum.wrapping_mul(31).wrapping_add(byte as u32);
    }
    (sum % 360) as f32
}

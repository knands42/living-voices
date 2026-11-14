use crate::backend::domain::adventure::AdventureSummary;
use crate::ui::pages::main_hub::{AdventureCreationDraft, Message, State};
use crate::ui::{common, theme};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};

const RAIL_LENGTH: f32 = 320.0;
const PLATE_LENGTH: f32 = 292.0;

pub fn body(home_state: &State) -> Element<'_, Message> {
    let main = column![
        newest_adventures(home_state),
        shelf(home_state),
        catalogue(home_state)
    ]
    .spacing(18.0)
    .width(Length::Fill);

    let rail = column![create_card(), one_shot(home_state), numbers()]
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

fn newest_adventures(home_state: &State) -> Element<'_, Message> {
    let Some(newest) = AdventureSummary::newest(&home_state.catalogue.adventures) else {
        None
    };
    let accent = theme::hsv(newest.genre.hue(), 0.45, 0.82);

    // The plate stands in for the cover: the cast you are about to meet, drawn
    // in their own hues over the story's tint.
    let plate = container(
        column![
            text("IN THIS STORY")
                .size(9.5)
                .color(theme::alpha(theme::TEXT_LIGHT, 0.55))
                .font(common::semibold()),
            Space::new().height(Length::Fill),
            row![
                common::initials(spot.initials[0], newest.genre.hue() + 22.0, 56.0, 0.7),
                common::initials(spot.initials[1], card.genre.hue() + 52.0, 44.0, 0.4),
                common::initials(spot.initials[2], card.genre.hue() - 26.0, 44.0, 0.3),
            ]
            .spacing(6.0)
            .align_y(Alignment::Center),
            text(spot.cast_note)
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
    .style(theme::cover(card.genre.hue()));

    let facts = row![
        text(card.genre.label().to_uppercase())
            .size(9.5)
            .color(accent)
            .font(common::semibold()),
        common::dot(4.0, theme::alpha(accent, 0.7)),
        text(format!(
            "{} chapters · {} voices · {} players",
            card.chapters,
            card.voices,
            compact(card.players)
        ))
        .size(10.5)
        .color(theme::TEXT_DIM),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center);

    let body = column![
        facts,
        text(card.title)
            .size(32.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(card.premise)
            .size(14.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        common::divider(),
        text(spot.bill)
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
            text(format!("a story by {}", card.author))
                .size(10.5)
                .color(theme::TEXT_DIM),
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
    let header = row![
        common::heading("Pick up where you left off"),
        Space::new().width(Length::Fill),
        text(format!("{} open", home_state.catalogue.shelf.len()))
            .size(10.5)
            .color(theme::TEXT_DIM),
    ]
    .align_y(Alignment::Center);

    let mut cards = row![].spacing(12.0).width(Length::Fill);
    for (index, entry) in home_state.catalogue.shelf.iter().enumerate() {
        cards = cards.push(shelf_card(home, index, entry));
    }

    column![header, cards]
        .spacing(10.0)
        .width(Length::Fill)
        .into()
}

fn shelf_card<'a>(home_state: &'a Home, index: usize, entry: &'a Shelf) -> Element<'a, Message> {
    let card = &home_state.catalogue.cards[entry.card];
    let glow = home_state.glow_shelf.get(index).copied().unwrap_or(0.0);
    let accent = theme::hsv(card.genre.hue(), 0.42, 0.80);

    let cover = container(
        text(card.title.chars().next().unwrap_or('·').to_string())
            .size(18.0)
            .color(theme::alpha(theme::TEXT_LIGHT, 0.85))
            .font(common::brand()),
    )
    .width(Length::Fixed(44.0))
    .height(Length::Fixed(44.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(theme::cover(card.genre.hue()));

    let head = row![
        cover,
        column![
            text(card.title)
                .size(13.5)
                .color(theme::TEXT_LIGHT)
                .font(common::semibold())
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text(format!(
                "act {} of {} · {}",
                entry.act, entry.acts, entry.when
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
            entry.progress,
            theme::alpha(accent, 0.95),
            theme::alpha(theme::BRONZE_DIM, 0.55),
            3.0,
        ),
        text(format!("{:.0}%", entry.progress * 100.0))
            .size(10.0)
            .color(theme::TEXT_DIM),
    ]
    .spacing(9.0)
    .align_y(Alignment::Center);

    mouse_area(
        container(column![head, foot].spacing(11.0).width(Length::Fill))
            .padding(12.0)
            .width(Length::Fill)
            .style(theme::shelf_card(card.genre.hue(), glow)),
    )
    .on_press(Message::Home(HomeMessage::Open(entry.card)))
    .on_enter(Message::Home(HomeMessage::ShelfHovered(Some(index))))
    .on_exit(Message::Home(HomeMessage::ShelfHovered(None)))
    .into()
}

// --- the catalogue --------------------------------------------------------

fn catalogue(home_state: &State) -> Element<'_, Message> {
    let catalog = home_state.catalog();

    let header = row![
        common::heading("The library"),
        text(format!(
            "{} of {}",
            catalog.len(),
            home_state.catalogue.cards.len()
        ))
        .size(10.5)
        .color(theme::TEXT_DIM),
        Space::new().width(Length::Fill),
        sort_tabs(home),
    ]
    .spacing(10.0)
    .align_y(Alignment::Center);

    let mut filters = row![].spacing(7.0);
    filters = filters.push(chip_button(home, None, "All"));
    for genre in Genre::ALL {
        filters = filters.push(chip_button(home, Some(genre), genre.label()));
    }
    if home_state.filtered() {
        filters = filters.push(
            button(text("clear").size(10.5))
                .padding([7.0, 10.0])
                .style(theme::sort_tab(false))
                .on_press(Message::Home(HomeMessage::ClearFilters)),
        );
    }

    let bar = row![filters, Space::new().width(Length::Fill)]
        .width(Length::Fill)
        .align_y(Alignment::Center);

    let mut list = column![].spacing(7.0).width(Length::Fill);
    for (index, card) in home_state.catalogue.enumerate() {
        list = list.push(row_card(home_state, index, card));
    }

    column![header, bar, list]
        .spacing(12.0)
        .width(Length::Fill)
        .into()
}

fn row_card<'a>(
    home_state: &'a State,
    index: usize,
    card: &'a AdventureSummary,
) -> Element<'a, Message> {
    let glow = home_state.glow.get(index).copied().unwrap_or(0.0);

    let cover = container(
        text(card.title.chars().next().unwrap_or('·').to_string())
            .size(20.0)
            .color(theme::alpha(theme::TEXT_LIGHT, 0.85))
            .font(common::brand()),
    )
    .width(Length::Fixed(56.0))
    .height(Length::Fixed(56.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(theme::cover(card.tone.hue()));

    let mut title = row![
        text(card.title)
            .size(15.5)
            .color(theme::TEXT_LIGHT)
            .font(common::semibold()),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center);

    if card.fresh {
        title = title.push(badge("new", theme::SEA));
    }
    if card.hot {
        title = title.push(badge("trending", theme::AMBER));
    }

    let body = column![
        title,
        text(card.premise)
            .size(12.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        text(format!(
            "{} · {} · ~{} min",
            card.tags.join(" · "),
            card.genre.label(),
            card.minutes
        ))
        .size(10.0)
        .color(theme::TEXT_DIM)
        .width(Length::Fill)
        .wrapping(Wrapping::Word),
    ]
    .spacing(4.0)
    .width(Length::Fill);

    let meta = column![
        text(compact(card.players))
            .size(12.0)
            .color(theme::TEXT_LIGHT)
            .font(common::semibold()),
        text("players").size(9.5).color(theme::TEXT_DIM),
        text(format!("{} ch", card.chapters))
            .size(9.5)
            .color(theme::alpha(theme::TEXT_MUTED, 0.9)),
    ]
    .spacing(1.0)
    .align_x(Alignment::End)
    .width(Length::Fixed(56.0));

    let open = container(
        text("→")
            .size(13.0)
            .color(theme::alpha(theme::TEXT_LIGHT, 0.35 + 0.55 * glow)),
    )
    .width(Length::Fixed(26.0))
    .align_x(Alignment::Center);

    mouse_area(
        container(
            row![cover, container(body).width(Length::Fill), meta, open,]
                .spacing(14.0)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        )
        .padding([10.0, 13.0])
        .width(Length::Fill)
        .style(theme::library_row(glow, card.genre.hue())),
    )
    .on_press(Message::Home(HomeMessage::Open(index)))
    .on_enter(Message::Home(HomeMessage::Hovered(Some(index))))
    .on_exit(Message::Home(HomeMessage::Hovered(None)))
    .into()
}

fn empty_state() -> Element<'static, Message> {
    container(
        column![
            text("Nothing on the shelf matches")
                .size(13.5)
                .color(theme::TEXT_LIGHT),
            text("Try a wider word, or hand the filter back.")
                .size(11.5)
                .color(theme::TEXT_DIM),
            button(text("Clear the filter").size(11.5))
                .padding([7.0, 12.0])
                .style(theme::quick_chip)
                .on_press(Message::Home(HomeMessage::ClearFilters)),
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

fn sort_tabs(home_state: &State) -> Element<'_, Message> {
    let mut tabs = row![].spacing(3.0).align_y(Alignment::Center);
    for sort in Sort::ALL {
        tabs = tabs.push(
            button(text(sort.label()).size(10.5))
                .padding([5.0, 9.0])
                .style(theme::sort_tab(home_state.sort == sort))
                .on_press(Message::Home(HomeMessage::SortPicked(sort))),
        );
    }
    tabs.into()
}

fn chip_button<'a>(
    home_state: &State,
    genre: Option<Genre>,
    label: &'a str,
) -> Element<'a, Message> {
    let selected = home_state.genre == genre;

    button(text(label).size(12.0))
        .padding([7.0, 13.0])
        .style(theme::chip(selected))
        .on_press(Message::Home(HomeMessage::GenrePicked(genre)))
        .into()
}

fn badge(label: &'static str, color: iced::Color) -> Element<'static, Message> {
    container(text(label).size(9.0).color(color).font(common::semibold()))
        .padding([2.0, 7.0])
        .style(theme::badge(color))
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

fn start_option(start: StartFrom, selected: bool) -> Element<'static, Message> {
    let radio = container(
        Space::new()
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0)),
    )
    .style(move |_theme| theme::stage_dot(theme::GOLD, selected));

    button(
        row![
            radio,
            column![
                text(start.label())
                    .size(11.5)
                    .color(if selected {
                        theme::TEXT_LIGHT
                    } else {
                        theme::TEXT_MUTED
                    })
                    .font(common::semibold()),
                text(start.hint()).size(9.5).color(theme::TEXT_DIM),
            ]
            .spacing(1.0),
        ]
        .spacing(9.0)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding([8.0, 10.0])
    .width(Length::Fill)
    .style(theme::start_row(selected))
    .on_press(Message::Home(HomeMessage::StartFromPicked(start)))
    .into()
}

fn one_shot(home_state: &State) -> Element<'_, Message> {
    let shot = &home_state.catalogue.one_shot;

    container(
        column![
            common::eyebrow("one-shot tonight"),
            text(shot.title)
                .size(16.0)
                .color(theme::TEXT_LIGHT)
                .font(common::brand()),
            text(shot.hook)
                .size(11.0)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            row![
                text(format!("~{} min · solo · no save", shot.minutes))
                    .size(9.5)
                    .color(theme::TEXT_DIM),
                Space::new().width(Length::Fill),
                button(text("Deal me one").size(11.0))
                    .padding([6.0, 11.0])
                    .style(theme::quick_chip)
                    .on_press(Message::Home(HomeMessage::Surprise)),
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill),
        ]
        .spacing(8.0)
        .width(Length::Fill),
    )
    .padding(14.0)
    .width(Length::Fixed(RAIL_LENGTH))
    .style(theme::inset)
    .into()
}

// TODO: Make this dynamic
fn numbers() -> Element<'_, Message> {
    column![
        common::eyebrow("your shelf"),
        row![
            stat_cell("stories", 4.to_string()),
            stat_cell("hours", format!("{:.1}", 6.50)),
            stat_cell("nights", 3.to_string()),
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

// --- the create adventure pop-up -----------------------------------------------------

pub fn new_adventure_popup(home_state: &State) -> Element<'_, Message> {
    let mut adventure_creation_draft = AdventureCreationDraft::default();

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
            text_input("The Falling Stars", &adventure_creation_draft.title)
                .on_input(|value| {
                    adventure_creation_draft.title = value;
                    Message::AdventureDraftChanged(adventure_creation_draft.clone())
                })
                .on_submit(Message::AdventureDraftSubmitted)
                .padding([11.0, 13.0])
                .size(13.0)
                .style(theme::composer),
            text("Headline - One true sentence about the world is enough to start")
                .size(12.0)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text_input(
                "A harbour town where the tide keeps the debts",
                &adventure_creation_draft.headline
            )
            .on_input(|value| Message::AdventureDraftChanged({
                adventure_creation_draft.headline = value;
                adventure_creation_draft.clone()
            }))
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

    let dim = container(mouse_area(card).on_press(Message::AdventureDraftClosedNoop))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(theme::lightbox);

    mouse_area(dim)
        .on_press(Message::AdventureDraftClosedNoop)
        .into()
}

// --- small things ---------------------------------------------------------

/// 8420 -> "8.4k", the way every storefront writes a play count.
fn compact(n: u32) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f32 / 1_000.0)
    } else {
        n.to_string()
    }
}

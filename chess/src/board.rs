use bevy::prelude::*;

use crate::piece::{Piece, Turn};

const BROWN_COLOR: Color = Color::rgb(181.0 / 255.0, 136.0 / 255.0, 99.0 / 255.0);
const LIGTH_BROWN_COLOR: Color = Color::rgb(240.0 / 255.0, 217.0 / 255.0, 181.0 / 255.0);

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<Turn>()
            .add_event::<ResetSelectedEvent>()
            .add_event::<ResetHighlightedSquares>()
            .add_startup_system(create_board)
            .add_system(despawn_taken_pieces)
            .add_system(select_square.label("select_square"))
            .add_system(
                // move_piece needs to run before select_piece
                move_piece
                    .after("select_square")
                    .before("select_piece"),
            )
            .add_system(
                select_piece
                    .after("select_square")
                    .label("select_piece"),
            )
            .add_system(highlight_squares)
            .add_system(reset_selected.after("select_square"))
            .add_system(reset_highlighted.after("select_square"));
    }
}

#[derive(Component, Debug)]
struct Square {
    x: u8,
    y: u8,
}

// Point struct used for piece highlighting so you can iterate over the points and remove them after moving
#[derive(Component)]
struct Point;

// To mark a square to be highlighted
#[derive(Component)]
struct Highlight;

#[derive(Component)]
struct Taken;

#[derive(Default, Debug)]
struct SelectedSquare {
    entity: Option<Entity>
}

#[derive(Default)]
struct SelectedPiece {
    entity: Option<Entity>
}

struct ResetSelectedEvent;
struct ResetHighlightedSquares;

impl Square {
    fn new(x: u8, y: u8) -> Self {
        Self {
            x,
            y
        }
    }
}

fn select_square(
    mouse_button_inputs: Res<Input<MouseButton>>,
    mut selected_square: ResMut<SelectedSquare>,
    mut squares_query: Query<(Entity, &Square)>,
    windows: Res<Windows>,
    mut reset_highlighted_event: EventWriter<ResetHighlightedSquares>
) {
    // Only run if the left button is pressed
    if !mouse_button_inputs.just_pressed(MouseButton::Left) {
        return;
    }

    let window: &Window = windows.get_primary().unwrap();

    if let Some(pos) = window.cursor_position() {
        let x: u8 = (pos.x / super::SQUARE_SIZE) as u8;
        let y: u8 = (pos.y / super::SQUARE_SIZE) as u8;

        squares_query.for_each_mut( | (entity, square) | {
            if square.x == x && square.y == y {
                selected_square.entity = Some(entity);
            }
        });
    }

    reset_highlighted_event.send(ResetHighlightedSquares);
}

fn select_piece(
    selected_square: Res<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    turn: Res<Turn>,
    square_query: Query<(Entity, &Square)>,
    piece_entitys: Query<(Entity, &Piece)>,
    mut commands: Commands
) {
    // if square is not changed the square can't be valid
    if !selected_square.is_changed() {
        return;
    }

    let square_entity: Entity = if let Some(entity) = selected_square.entity {
        entity 
    } else {
        return;
    };

    let square: &Square = if let Ok((_, square)) = square_query.get(square_entity) {
        square
    } else {
        return;
    };

    let mut pieces_on_the_board: Vec<Piece> = Vec::new();

    for (_, piece) in piece_entitys.iter() {
        pieces_on_the_board.push(piece.clone());
    }

    if selected_piece.entity.is_none() {
        for (entity, piece) in piece_entitys.iter() {
            if piece.pos.0 as u8 == square.x && piece.pos.1 as u8 == square.y && piece.color == turn.0 {
                selected_piece.entity = Some(entity);

                for i in 0..pieces_on_the_board.len() {
                    if pieces_on_the_board[i].pos.0 == piece.pos.0 && pieces_on_the_board[i].pos.1 == piece.pos.1 {
                        pieces_on_the_board.remove(i);
                        break;
                    }
                }

                let positions: Vec<(u8, u8)> = piece.get_moves(&pieces_on_the_board);

                for i in 0..positions.len() {
                    for (entity, square) in square_query.iter() {
                        if square.x == positions[i].0 && square.y == positions[i].1 {
                            commands.entity(entity).insert(Highlight);
                        }
                    }
                }

                break;
            }
        }
    }
}

fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    mut turn: ResMut<Turn>,
    squares_query: Query<&Square>,
    mut pieces_query: Query<(Entity, &mut Piece)>,
    mut reset_selected_event: EventWriter<ResetSelectedEvent>,
    mut reset_highlighted_event: EventWriter<ResetHighlightedSquares>
) {
    if !selected_square.is_changed() {
        return;
    }

    // Removing the highlighted spots from the board
    reset_highlighted_event.send(ResetHighlightedSquares);

    let square_entity = if let Some(entity) = selected_square.entity {
        entity
    } else {
        return;
    };

    let square = if let Ok(square) = squares_query.get(square_entity) {
        square
    } else {
        return;
    };

    if let Some(selected_piece_entity) = selected_piece.entity {
        let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, piece)| piece.clone()).collect();
        let pieces_entity_vec: Vec<(Entity, Piece)> = pieces_query
            .iter_mut()
            .map(|(entity, piece)| (entity, piece.clone()))
            .collect::<Vec<(Entity, Piece)>>();

        // Move the selected piece to the selected square
        let mut piece = if let Ok((_piece_entity, piece)) = pieces_query.get_mut(selected_piece_entity) {
            piece
        } else {
            return;
        };

        if piece.is_move_valid((square.x, square.y), &pieces_vec) {
            for (other_entity, other_piece) in pieces_entity_vec {
                if other_piece.pos.0 == square.x    
                    && other_piece.pos.1 == square.y
                    && other_piece.color != piece.color
                {
                    // Mark the piece as taken
                    commands.entity(other_entity).insert(Taken);
                }
            }
    
            // Move piece
            piece.pos.0 = square.x;
            piece.pos.1 = square.y;
    
            // Change turn
            turn.change();
        }

        reset_selected_event.send(ResetSelectedEvent);
    }
}

fn reset_selected(
    mut event_reader: EventReader<ResetSelectedEvent>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
) {
    for _event in event_reader.iter() {
        selected_square.entity = None;
        selected_piece.entity = None;
    }
}

fn reset_highlighted(
    point_query: Query<(Entity, &Point)>,
    square_query: Query<(Entity, &Square, &Highlight)>,
    mut commands: Commands,
    mut event_reader: EventReader<ResetHighlightedSquares>
) {
    for _event in event_reader.iter() {
        for (entity, _, _) in square_query.iter() {
            commands.entity(entity).remove::<Highlight>();
        }

        for (entity, _) in point_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn despawn_taken_pieces(
    mut commands: Commands,
    query: Query<(Entity, &Piece, &Taken)>
) {
    // Gets all pieces that are marked as taken and removes them

    for (entity, _, _) in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn highlight_squares(
    query: Query<(&Square, &Highlight)>,
    mut commands: Commands
) {
    query.for_each( | (square, _) | {
        let position = Vec3::new(super::OFFSET + square.x as f32 * super::SQUARE_SIZE, super::OFFSET + square.y as f32 * super::SQUARE_SIZE, 1.0);

        commands
            .spawn()
            .insert(Point)
            .insert_bundle( SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(1.0, 0.0, 0.0),
                    ..default()
                },
                transform: Transform {
                    translation: position,
                    scale: Vec3::new(super::SQUARE_SIZE, super::SQUARE_SIZE, 2.0),
                    ..default()
                },
                ..default()
        });
    });
}

fn create_board(
    mut commands: Commands
)   {
    // Cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    // Create Chessboard 8x8
    for row in 0..8 {
        for column in 0..8 {
            let square_position = Vec2::new(
                super::OFFSET + column as f32 * (super::SQUARE_SIZE),
                super::OFFSET + row as f32 * (super::SQUARE_SIZE),
            );

            if (row + column) % 2 != 0 {
                // Insert brown square
                commands
                .spawn()
                .insert(Square::new(row, column))
                .insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: LIGTH_BROWN_COLOR,
                        ..default()
                    },
                    transform: Transform {
                        translation: square_position.extend(0.0),
                        scale: Vec3::new(super::SQUARE_SIZE, super::SQUARE_SIZE, 0.0),
                        ..default()
                    },
                    ..default()
                });
            }
            else {
                // Insert white square 
                commands
                .spawn()
                .insert(Square::new(row, column))
                .insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: BROWN_COLOR,
                        ..default()
                    },
                    transform: Transform {
                        translation: square_position.extend(0.0),
                        scale: Vec3::new(super::SQUARE_SIZE, super::SQUARE_SIZE, 1.0),
                        ..default()
                    },
                    ..default()
                });
            }
        }
    }
}
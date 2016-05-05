extern crate rand;

use types::*;

pub fn new_actions() -> Vec<Action> {
    let actions: Vec<Action> = Vec::new();
    actions
}

pub fn new_game() -> Game {
    let all_tiles = (0..TILES).map(|i| {
        let row = i / COLS;
        let col = i % ROWS;
        if let Some(tile) = Tile::new(row, col) {
            tile
        } else {
            panic!("Attempted to create invalid tile ({},{})", row, col)
        }
        }).collect();
    let (starting_tiles, remaining_tiles) = choose_tiles(all_tiles, PLAYERS);
    let players = new_players(remaining_tiles);
    let slots = initial_slots(starting_tiles);
    Game {
        board: Board { slots: slots },
        players: players, 
        turn: PlayerId::One, 
        merge_decision: None
    }
}

fn choose_tiles(tiles: Vec<Tile>, count: u8) -> (Vec<Tile>, Vec<Tile>) {
    let mut remaining_tiles = tiles;
    let mut random_tiles = Vec::new();
    for _ in 0..count {
        let random_index = rand::random::<usize>() % remaining_tiles.len();
        random_tiles.push(remaining_tiles.remove(random_index));
    }
    (random_tiles, remaining_tiles)
}

fn new_players(tiles: Vec<Tile>) -> Vec<Player>{
    let init_players: Vec<Player> = Vec::new();
    let (players, _) = (0..PLAYERS)
        .fold( (init_players, tiles), | (mut v, remaining), i | {
            let (player_tiles, new_remaining) = choose_tiles(remaining, 6);
            let player = new_player(PlayerId::new(i+1).unwrap(), player_tiles);
            v.push(player);
            (v, new_remaining)
        });
    players
}

pub fn new_player(id: PlayerId, tiles: Vec<Tile>) -> Player {
    Player { id: id, money: 6000, shares: empty_shares(), tiles: tiles }
}

fn empty_shares() -> PlayerShares {
    PlayerShares { luxor: 0, tower: 0, american: 0, festival: 0, worldwide: 0, continental: 0, imperial: 0 }
}

pub fn initial_slots(starting_tiles: Vec<Tile>) -> Vec<Slot> {
    (0..ROWS).flat_map(|row| -> Vec<Slot> {
        (0..COLS).map(|col| {
            Slot { 
                row: row, 
                col: col, 
                hotel: None, 
                has_tile: has_tile_on_slot(&starting_tiles, row, col)
            } 
        }).collect()
    }).collect()
}

fn has_tile_on_slot(tiles: & Vec<Tile>, row: u8, col: u8) -> bool {
    tiles.iter().any(|t| t.row() == row && t.col() == col)
}

pub fn compute_state(last_state: &Game, actions: &Vec<Action>) -> TurnResult {
    actions.iter().fold(TurnResult::Success(last_state.clone()), |last_result, action| {
        match last_result {
            TurnResult::Success(game) => { play_turn(&game, action) }
            TurnResult::Error(_) => { last_result }
        }
    })
}

pub fn play_turn(game: &Game, action: &Action) -> TurnResult {
    match *action {
        Action::PlaceTile { ref player, ref tile } => {
            place_tile(game, player.clone(), tile)
        }
        Action::BuyStocks { ref player, ref hotel1, ref hotel2, ref hotel3 } => {
            buy_stocks(game, player.clone(), hotel1.clone(), hotel2.clone(), hotel3.clone())
        }
        _ => panic!(format!("I don't know how to play a turn with action {:?}", action))
    }
}

fn place_tile(game: &Game, player_id: PlayerId, tile: &Tile) -> TurnResult {
    if !game_player_has_turn(game, player_id.clone()) {
        let error_msg = format!("Error placing tile: player {:?} does not have turn", player_id);
        return TurnResult::Error(error_msg)
    }
    if !game_player_has_tile(game, player_id.clone(), tile) {
        let error_msg = format!("Error placing tile: player {:?} does not have tile {:?}", player_id, *tile);
        return TurnResult::Error(error_msg)
    }
    let new_players = remove_tile_from_player(game.players.clone(), player_id.clone(), tile);
    TurnResult::Success(Game {
        board: place_tile_on_board(&game.board, &tile),
        players: new_players,
        turn: game.turn.clone(),
        merge_decision: game.merge_decision.clone()
    })
}

fn remove_tile_from_player(mut players: Vec<Player>, player_id: PlayerId, tile: &Tile) -> Vec<Player> {
    let player_index = players.iter().position(|p| p.id == player_id).unwrap();
    let tile_index = players[player_index].tiles.iter().position(|t| *t == *tile).unwrap();
    players[player_index].tiles.remove(tile_index);
    players
}

fn game_player_has_turn(game: &Game, player: PlayerId) -> bool {
    return game.turn == player
}

fn game_player_has_tile(game: &Game, player: PlayerId, tile: &Tile) -> bool {
    game.players.iter()
        .find(|p| p.id == player)
        .map_or(false, |p| player_has_tile(p, tile))
}

fn player_has_tile(player: &Player, tile: &Tile) -> bool {
    player.tiles.iter().any(|t| *t == *tile)
}

fn place_tile_on_board(board: &Board, tile: &Tile) -> Board {
    let slots = board.slots
        .iter()
        .map(|s| {
            if s.row == tile.row() && s.col == tile.col() {
                Slot { row: s.row, col: s.col, has_tile: true, hotel: s.hotel.clone() }
            } else {
                s.clone()
            }
        })
        .collect();
    Board { slots: slots }
}

fn buy_stocks(game: &Game, player: PlayerId, hotel1: Option<Hotel>, hotel2: Option<Hotel>, hotel3: Option<Hotel>) -> TurnResult {
    let new_players: Vec<Player> = game.players
        .iter()
        .map(|p| {
            if p.id == player {
                player_buy_stocks(&game, &p, hotel1.clone(), hotel2.clone(), hotel3.clone())
            } else {
                p.clone()
            }
        })
        .collect();
    TurnResult::Success(Game {
        board: game.board.clone(),
        players: new_players,
        turn: game.turn.clone(),
        merge_decision: game.merge_decision.clone()
    })
}

fn player_buy_stocks(game: &Game, player: &Player, hotel1: Option<Hotel>, hotel2: Option<Hotel>, hotel3: Option<Hotel>) -> Player {
  let new_shares = vec![hotel1.clone(), hotel2.clone(), hotel3.clone()]
        .iter()
        .fold(player.shares.clone(), | shares, hotel: &Option<Hotel> | {
            match *hotel {
                Some(ref h) => { add_share(shares, h.clone()) }
                None => { shares }
            }
        });
  let total_cost = share_price(game, hotel1) + share_price(game, hotel2) + share_price(game, hotel3);
  let money_after = player.money - total_cost;
  Player { 
      id: player.id.clone(), 
      money: money_after, 
      shares: new_shares, 
      tiles: player.tiles.clone()
  }
}

fn share_price(game: &Game, hotel: Option<Hotel>) -> i32 {
  hotel.map(|h| stock_price(h.clone(), hotel_chain_size(game, h.clone()))).unwrap_or(0)
}

fn add_share(shares: PlayerShares, hotel: Hotel) -> PlayerShares {
    let mut new_shares = PlayerShares {
        luxor: shares.luxor,
        tower: shares.tower,
        american: shares.american,
        festival: shares.festival,
        worldwide: shares.worldwide,
        continental: shares.continental,
        imperial: shares.imperial
    };
    match hotel {
        Hotel::Tower =>       { new_shares.tower += 1 }
        Hotel::Luxor =>       { new_shares.luxor += 1 }
        Hotel::American =>    { new_shares.american += 1 }
        Hotel::Worldwide =>   { new_shares.worldwide += 1 }
        Hotel::Festival =>    { new_shares.festival += 1 }
        Hotel::Imperial =>    { new_shares.imperial += 1 }
        Hotel::Continental => { new_shares.continental += 1 }
    };
    new_shares
}

fn hotel_chain_size(game: &Game, hotel: Hotel) -> u8 {
  2
}

fn stock_price(hotel: Hotel, num_tiles: u8) -> i32 {
    base_price(hotel) + 100 * price_level(num_tiles) as i32
}

fn base_price(hotel: Hotel) -> i32 {
    let cheap = 200;
    let medium = 300;
    let spendy = 400;
    match hotel {
        Hotel::Tower =>       { cheap }
        Hotel::Luxor =>       { cheap }
        Hotel::American =>    { medium }
        Hotel::Worldwide =>   { medium }
        Hotel::Festival =>    { medium }
        Hotel::Imperial =>    { spendy }
        Hotel::Continental => { spendy }
    }
}

fn price_level(num_tiles: u8) -> u8 {
  if num_tiles == 2 {
      0
  } else if num_tiles == 3 {
      1
  } else if num_tiles == 4 {
      2
  } else if num_tiles == 5 {
      3
  } else if num_tiles >= 6 && num_tiles <= 10 {
      4
  } else if num_tiles >= 11 && num_tiles <= 20 {
      5
  } else if num_tiles >= 21 && num_tiles <= 30 {
      6
  } else if num_tiles >= 31 && num_tiles <= 40 {
      7
  } else { // >= 41
      8
  }
}

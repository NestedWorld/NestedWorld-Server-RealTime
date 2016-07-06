#![allow(dead_code, unused_variables)]
use mioco::tcp::TcpStream;
use self::state::State;

pub mod store;

pub mod state {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum State {
        #[doc(hidden)]
        __InvalidState__,
        WaitingPlayers,
        WaitingAttack,
        MonsterKo(u32),
        Finished,
    }

    macro_rules! action {
        ($self_:expr, $($pattern:pat => $state:expr),*) => {{
            let new_state = match $self_.0 {
                $($pattern => Some($state),)*
                _ => None,
            };

            if let Some(state) = new_state {
                $self_.0 = state;
            }

            new_state
        }};
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Machine(State);

    impl Machine {
        pub fn new() -> Machine {
            Machine(State::WaitingPlayers)
        }

        pub fn state(&self) -> State {
            self.0
        }

        pub fn start(&mut self) -> Option<State> {
            action!(self,
                State::WaitingPlayers => State::WaitingAttack
            )
        }

        pub fn monster_ko(&mut self, id: u32) -> Option<State> {
            action!(self,
                State::WaitingAttack => State::MonsterKo(id)
            )
        }

        pub fn replace(&mut self) -> Option<State> {
            action!(self,
                State::MonsterKo(_) => State::WaitingAttack
            )
        }

        pub fn finish(&mut self) -> Option<State> {
            action!(self,
                State::WaitingAttack => State::Finished,
                State::MonsterKo(_) => State::Finished
            )
        }
    }

    pub fn new() -> Machine { Machine::new() }
}

pub struct Combat {
    db: ::db::Database,
    id: u32,
    state: state::Machine,
    monsters: Vec<Monster>,
    players: Vec<Player>,
}

impl Combat {
    pub fn new(db: ::db::Database, id: u32) -> Combat {
        Combat {
            db: db,
            id: id,
            state: state::new(),
            monsters: Vec::new(),
            players: Vec::new(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn state(&self) -> State {
        self.state.state()
    }

    pub fn monsters(&self) -> &[Monster] {
        &self.monsters
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn add_player(&mut self, player: PlayerData, monsters: &[::db::models::user_monster::UserMonster]) -> Vec<u32> {
        let player_id = self.players.len() as u32;
        let monster_ids: Vec<u32> = (self.monsters.len() as u32..self.monsters.len() as u32 + monsters.len() as u32).collect();

        for monster in monsters {
            let monster = Monster {
                user_monster: monster.clone(),
                player: player_id,
                hp: monster.monster.get().unwrap().hp as u32,
            };
            self.monsters.push(monster);
        }

        let player = Player::new(player, &monster_ids);
        self.players.push(player);

        monster_ids
    }

    pub fn start(&mut self) -> bool {
        self.state.start().is_some()
    }

    pub fn attack(&mut self, player: u32, target: u32, attack: u32) {
        let ref mut m_target = self.monsters[target as usize];

        let damages = if self.players[m_target.player as usize].is_user() {
            20
        } else {
            3
        };

        m_target.hp = m_target.hp.checked_sub(damages).unwrap_or(0);

        if m_target.hp == 0 {
            self.state.monster_ko(target);
        }
    }

    pub fn flee(&mut self, player: u32) {
        self.finish(None);
    }

    pub fn replace(&mut self, player: u32, monster: u32) {
    }

    pub fn finish(&mut self, winner: Option<u32>) {
        self.state.finish();
    }
}

pub struct Monster {
    pub user_monster: ::db::models::user_monster::UserMonster,
    pub player: u32,
    pub hp: u32,
}

pub struct Player {
    pub monsters: Vec<u32>,
    pub current_monster: u32,
    pub data: PlayerData,
}

pub enum PlayerData {
    User {
        user: ::db::models::user::User,
        stream: TcpStream,
    },
    AI,
}

impl PlayerData {
    pub fn is_user(&self) -> bool {
        match *self {
            PlayerData::User { .. } => true,
            _ => false,
        }
    }

    pub fn is_ai(&self) -> bool {
        match *self {
            PlayerData::AI => true,
            _ => false,
        }
     }
}

impl Player {
    pub fn new(data: PlayerData, monsters: &[u32]) -> Player {
        Player {
            monsters: monsters.to_owned(),
            current_monster: monsters[0],
            data: data,
        }
    }

    pub fn is_user(&self) -> bool {
        self.data.is_user()
    }

    pub fn is_ai(&self) -> bool {
        self.data.is_ai()
    }
}

use specs::prelude::*;

use galangua_common::app::consts::*;
use galangua_common::app::game::attack_manager::AttackManager;
use galangua_common::app::game::effect_table::to_earned_point_type;
use galangua_common::app::game::formation::Formation;
use galangua_common::app::game::formation_table::{X_COUNT, Y_COUNT};
use galangua_common::app::game::star_manager::StarManager;
use galangua_common::app::game::traj::Accessor as TrajAccessor;
use galangua_common::app::game::traj::Traj;
use galangua_common::app::game::traj_command::TrajCommand;
use galangua_common::app::game::traj_command_table::*;
use galangua_common::app::game::{EnemyType, FormationIndex};
use galangua_common::framework::types::{Vec2I, ZERO_VEC};
use galangua_common::util::math::{atan2_lut, calc_velocity, clamp, diff_angle, normalize_angle, square, ANGLE, ONE, ONE_BIT};

use crate::app::components::*;
use crate::app::resources::{EneShotSpawner, GameInfo};

use super::system_effect::*;
use super::system_owl::set_owl_damage;

struct Vtable {
    rush_traj_table: &'static [TrajCommand],
}

const VTABLE: [Vtable; 4] = [
    // Bee
    Vtable {
        rush_traj_table: &BEE_RUSH_ATTACK_TABLE,
    },
    // Butterfly
    Vtable {
        rush_traj_table: &BUTTERFLY_RUSH_ATTACK_TABLE,
    },
    // Owl
    Vtable {
        rush_traj_table: &OWL_RUSH_ATTACK_TABLE,
    },
    // CapturedFighter
    Vtable {
        rush_traj_table: &OWL_RUSH_ATTACK_TABLE,
    },
];

pub fn forward(posture: &mut Posture, speed: &Speed) {
    posture.0 += &calc_velocity(posture.1 + speed.1 / 2, speed.0);
    posture.1 += speed.1;
}

pub fn move_to_formation(posture: &mut Posture, speed: &mut Speed, fi: &FormationIndex, formation: &Formation) -> bool {
    let target = formation.pos(fi);
    let pos = &mut posture.0;
    let angle = &mut posture.1;
    let spd = &mut speed.0;
    let vangle = &mut speed.1;
    let diff = &target - &pos;
    let sq_distance = square(diff.x >> (ONE_BIT / 2)) + square(diff.y >> (ONE_BIT / 2));
    if sq_distance > square(*spd >> (ONE_BIT / 2)) {
        let dlimit: i32 = *spd * 5 / 3;
        let target_angle = atan2_lut(-diff.y, diff.x);
        let d = diff_angle(target_angle, *angle);
        *angle += clamp(d, -dlimit, dlimit);
        *vangle = 0;
        false
    } else {
        *pos = target;
        *spd = 0;
        *angle = normalize_angle(*angle);
        *vangle = 0;
        true
    }
}

pub fn set_enemy_damage<'a>(
    entity: Entity, power: u32, entities: &Entities<'a>,
    enemy_storage: &mut WriteStorage<'a, Enemy>,
    pos_storage: &mut WriteStorage<'a, Posture>,
    zako_storage: &ReadStorage<'a, Zako>,
    owl_storage: &mut WriteStorage<'a, Owl>,
    troops_storage: &mut WriteStorage<'a, Troops>,
    coll_rect_storage: &mut WriteStorage<'a, CollRect>,
    seqanime_storage: &mut WriteStorage<'a, SequentialSpriteAnime>,
    drawable_storage: &mut WriteStorage<'a, SpriteDrawable>,
    recaptured_fighter_storage: &mut WriteStorage<'a, RecapturedFighter>,
    player_storage: &mut WriteStorage<'a, Player>,
    tractor_beam_storage: &mut WriteStorage<'a, TractorBeam>,
    attack_manager: &mut AttackManager,
    star_manager: &mut StarManager,
    game_info: &mut GameInfo,
    player_entity: Entity,
) {
    let enemy_type = enemy_storage.get(entity).unwrap().enemy_type;
    let point = match enemy_type {
        EnemyType::Owl => {
            let owl = owl_storage.get_mut(entity).unwrap();
            set_owl_damage(
                owl, entity, power, entities, enemy_storage, troops_storage, pos_storage,
                coll_rect_storage, drawable_storage, recaptured_fighter_storage, player_storage,
                tractor_beam_storage, attack_manager, star_manager, game_info, player_entity)
        }
        _ => {
            let is_formation = zako_storage.get(entity).unwrap().state == ZakoState::Formation;
            let point = calc_zako_point(enemy_type, is_formation);
            assert!(point > 0);
            entities.delete(entity).unwrap();
            if enemy_type == EnemyType::CapturedFighter {
                game_info.captured_fighter_destroyed();
            }
            point
        }
    };
    if point > 0 {
        let pos = pos_storage.get(entity).unwrap().0.clone();

        if let Some(point_type) = to_earned_point_type(point) {
            create_earned_piont_effect(point_type, &pos, entities, pos_storage, seqanime_storage, drawable_storage);
        }

        create_enemy_explosion_effect(&pos, entities, pos_storage, seqanime_storage, drawable_storage);

        game_info.score_holder.add_score(point);
        game_info.decrement_alive_enemy();
    }
}

//

pub fn move_zako<'a>(
    zako: &mut Zako, entity: Entity, enemy: &mut Enemy, speed: &mut Speed,
    formation: &Formation, player_storage: &ReadStorage<'a, Player>,
    pos_storage: &mut WriteStorage<'a, Posture>,
    entities: &Entities<'a>, game_info: &mut GameInfo,
    eneshot_spawner: &mut EneShotSpawner,
) {
    match zako.state {
        ZakoState::Appearance => {
            let mut accessor = EneBaseAccessorImpl::new(formation, eneshot_spawner, game_info.stage);
            if !zako.base.update_trajectory(pos_storage.get_mut(entity).unwrap(), speed, &mut accessor) {
                zako.base.traj = None;
                if enemy.formation_index.1 >= Y_COUNT as u8 {  // Assault
                    zako.base.set_assault(speed, player_storage, pos_storage);
                    zako.state = ZakoState::Assault(0);
                } else {
                    zako.state = ZakoState::MoveToFormation;
                }
            }
        }
        ZakoState::Formation => {
            let mut posture = pos_storage.get_mut(entity).unwrap();
            posture.0 = formation.pos(&enemy.formation_index);

            let ang = ANGLE * ONE / 128;
            posture.1 -= clamp(posture.1, -ang, ang);
        }
        ZakoState::Attack(t) => {
            let mut accessor = EneBaseAccessorImpl::new(formation, eneshot_spawner, game_info.stage);
            zako.base.update_attack(&pos_storage.get(entity).unwrap().0, &mut accessor);
            match t {
                ZakoAttackType::BeeAttack => {
                    update_bee_attack(
                        zako, enemy, pos_storage.get_mut(entity).unwrap(), speed, formation,
                        game_info, eneshot_spawner);
                }
                ZakoAttackType::Traj => {
                    update_attack_traj(
                        zako, enemy, pos_storage.get_mut(entity).unwrap(), speed, formation, game_info, entity, entities,
                        eneshot_spawner);
                }
            }
        }
        ZakoState::MoveToFormation => {
            let posture = pos_storage.get_mut(entity).unwrap();
            let result = move_to_formation(posture, speed, &enemy.formation_index, formation);
            forward(posture, speed);
            if result {
                zako.state = ZakoState::Formation;
                enemy.is_formation = true;
            }
        }
        ZakoState::Assault(phase) => {
            let posture = pos_storage.get_mut(entity).unwrap();
            if let Some(new_phase) = zako.base.update_assault(posture, phase, entity, entities, game_info) {
                zako.state = ZakoState::Assault(new_phase);
            }
            forward(posture, speed);
        }
        ZakoState::Troop => {
            // Controlled by leader.
        }
    }
}

pub fn zako_start_attack(zako: &mut Zako, enemy: &mut Enemy, posture: &Posture) {
    let flip_x = enemy.formation_index.0 >= (X_COUNT as u8) / 2;
    let (table, state): (&[TrajCommand], ZakoState) = match enemy.enemy_type {
        EnemyType::Bee => (&BEE_ATTACK_TABLE, ZakoState::Attack(ZakoAttackType::BeeAttack)),
        EnemyType::Butterfly => (&BUTTERFLY_ATTACK_TABLE, ZakoState::Attack(ZakoAttackType::Traj)),
        EnemyType::Owl => (&OWL_ATTACK_TABLE, ZakoState::Attack(ZakoAttackType::Traj)),
        EnemyType::CapturedFighter => (&OWL_ATTACK_TABLE, ZakoState::Attack(ZakoAttackType::Traj)),
    };
    let mut traj = Traj::new(table, &ZERO_VEC, flip_x, enemy.formation_index.clone());
    traj.set_pos(&posture.0);

    zako.base.count = 0;
    zako.base.attack_frame_count = 0;
    zako.base.traj = Some(traj);
    zako.state = state;
    enemy.is_formation = false;
}

fn update_bee_attack<'a>(
    zako: &mut Zako, enemy: &Enemy, posture: &mut Posture, speed: &mut Speed, formation: &Formation,
    game_info: &GameInfo,
    eneshot_spawner: &mut EneShotSpawner,
) {
    let mut accessor = EneBaseAccessorImpl::new(formation, eneshot_spawner, game_info.stage);
    if !zako.base.update_trajectory(posture, speed, &mut accessor) {
        if game_info.is_rush() {
            let flip_x = enemy.formation_index.0 >= 5;
            let mut traj = Traj::new(&BEE_ATTACK_RUSH_CONT_TABLE, &ZERO_VEC, flip_x,
                                     enemy.formation_index);
            traj.set_pos(&posture.0);

            zako.base.traj = Some(traj);
            zako.state = ZakoState::Attack(ZakoAttackType::Traj);
        } else {
            zako.base.traj = None;
            zako.state = ZakoState::MoveToFormation;
        }
    }
}

fn update_attack_traj<'a>(
    zako: &mut Zako, enemy: &Enemy, posture: &mut Posture, speed: &mut Speed,
    formation: &Formation, game_info: &mut GameInfo, entity: Entity,
    entities: &Entities<'a>,
    eneshot_spawner: &mut EneShotSpawner,
) {
    let mut accessor = EneBaseAccessorImpl::new(formation, eneshot_spawner, game_info.stage);
    if !zako.base.update_trajectory(posture, speed, &mut accessor) {
        zako.base.traj = None;
        if enemy.enemy_type == EnemyType::CapturedFighter {
            entities.delete(entity).unwrap();
            game_info.decrement_alive_enemy();
        } else if game_info.is_rush() {
            // Rush mode: Continue attacking
            let table = VTABLE[enemy.enemy_type as usize].rush_traj_table;
            zako.base.rush_attack(table, posture, &enemy.formation_index);
            //accessor.push_event(EventType::PlaySe(CH_ATTACK, SE_ATTACK_START));
        } else {
            zako.state = ZakoState::MoveToFormation;
        }
    }
}

pub fn set_zako_to_troop(zako: &mut Zako, enemy: &mut Enemy) {
    zako.state = ZakoState::Troop;
    enemy.is_formation = false;
}

fn calc_zako_point(enemy_type: EnemyType, is_formation: bool) -> u32 {
    match enemy_type {
        EnemyType::Bee => {
            if is_formation { 50 } else { 100 }
        }
        EnemyType::Butterfly => {
            if is_formation { 80 } else { 160 }
        }
        EnemyType::CapturedFighter => {
            if is_formation { 500 } else { 1000 }
        }
        _ => { panic!("Illegal"); }
    }
}

//

pub fn move_eneshot<'a>(shot: &mut EneShot, posture: &mut Posture, entity: Entity, entities: &Entities<'a>) {
    posture.0 += &shot.0;
    if out_of_screen(&posture.0) {
        entities.delete(entity).unwrap();
    }
}

fn out_of_screen(pos: &Vec2I) -> bool {
    pos.x < -16 * ONE || pos.x > (WIDTH + 16) * ONE ||
        pos.y < -16 * ONE || pos.y > (HEIGHT + 16) * ONE
}

//

pub trait EneBaseAccessorTrait {
    fn fire_shot(&mut self, pos: &Vec2I);
    fn traj_accessor<'a>(&'a mut self) -> Box<dyn TrajAccessor + 'a>;
    fn get_stage_no(&self) -> u16;
}

impl EnemyBase {
    pub fn new(traj: Option<Traj>) -> Self {
        Self {
            traj,
            shot_wait: None,
            target_pos: ZERO_VEC,
            count: 0,
            attack_frame_count: 0,
        }
    }

    pub fn update_trajectory<A: EneBaseAccessorTrait>(&mut self, posture: &mut Posture, vel: &mut Speed, accessor: &mut A) -> bool {
        if let Some(traj) = &mut self.traj {
            let cont = traj.update(&*accessor.traj_accessor());
            posture.0 = traj.pos();
            posture.1 = traj.angle;
            vel.0 = traj.speed;
            vel.1 = traj.vangle;
            if let Some(wait) = traj.is_shot() {
                self.shot_wait = Some(wait);
            }

            if let Some(wait) = self.shot_wait {
                if wait > 0 {
                    self.shot_wait = Some(wait - 1);
                } else {
                    accessor.fire_shot(&posture.0);
                    self.shot_wait = None;
                }
            }

            if cont {
                return true;
            }
            self.traj = None;
        }
        false
    }

    pub fn update_assault<'a>(&mut self, posture: &mut Posture, phase: u32, entity: Entity, entities: &Entities<'a>, game_info: &mut GameInfo) -> Option<u32> {
        let pos = &mut posture.0;
        let angle = &mut posture.1;
        match phase {
            0 => {
                let target = &self.target_pos;
                let diff = target - pos;

                const DLIMIT: i32 = 5 * ONE;
                let target_angle = atan2_lut(-diff.y, diff.x);
                let d = diff_angle(target_angle, *angle);
                if d < -DLIMIT {
                    *angle -= DLIMIT;
                } else if d > DLIMIT {
                    *angle += DLIMIT;
                } else {
                    *angle += d;
                    return Some(1);
                }
            }
            1 | _ => {
                if pos.y >= (HEIGHT + 8) * ONE {
                    entities.delete(entity).unwrap();
                    game_info.decrement_alive_enemy();
                }
            }
        }
        None
    }

    pub fn update_attack<A: EneBaseAccessorTrait>(&mut self, pos: &Vec2I, accessor: &mut A) -> bool {
        self.attack_frame_count += 1;

        let stage_no = accessor.get_stage_no();
        let shot_count = std::cmp::min(2 + stage_no / 8, 5) as u32;
        let shot_interval = 20 - shot_count * 2;

        if self.attack_frame_count <= shot_interval * shot_count &&
            self.attack_frame_count % shot_interval == 0
        {
            accessor.fire_shot(pos);
            true
        } else {
            false
        }
    }

    pub fn rush_attack(&mut self, table: &'static [TrajCommand], posture: &Posture, fi: &FormationIndex) {
        let flip_x = fi.0 >= 5;
        let mut traj = Traj::new(table, &ZERO_VEC, flip_x, *fi);
        traj.set_pos(&posture.0);

        self.count = 0;
        self.attack_frame_count = 0;
        self.traj = Some(traj);
    }

    pub fn set_assault<'a>(&mut self, speed: &mut Speed, player_storage: &ReadStorage<'a, Player>, pos_storage: &WriteStorage<'a, Posture>) {
        /*let mut rng = Xoshiro128Plus::from_seed(rand::thread_rng().gen());
        let target_pos = [
            Some(*accessor.get_player_pos()),
            accessor.get_dual_player_pos(),
        ];
        let count = target_pos.iter().flat_map(|x| x).count();
        let target: &Vec2I = target_pos.iter()
            .flat_map(|x| x).nth(rng.gen_range(0, count)).unwrap();*/

        for (_player, posture) in (player_storage, pos_storage).join() {
            self.target_pos = posture.0.clone();
            speed.1 = 0;
            break;
        }
    }
}

pub struct EneBaseAccessorImpl<'l> {
    pub formation: &'l Formation,
    pub eneshot_spawner: &'l mut EneShotSpawner,
    pub stage_no: u16,
}

impl<'l> EneBaseAccessorImpl<'l> {
    pub fn new(formation: &'l Formation, eneshot_spawner: &'l mut EneShotSpawner, stage_no: u16) -> Self {
        Self {
            formation,
            eneshot_spawner,
            stage_no,
        }
    }
}

impl<'a> EneBaseAccessorTrait for EneBaseAccessorImpl<'a> {
    fn fire_shot(&mut self, pos: &Vec2I) {
        self.eneshot_spawner.push(pos);
    }

    fn traj_accessor<'b>(&'b mut self) -> Box<dyn TrajAccessor + 'b> {
        Box::new(TrajAccessorImpl { formation: self.formation, stage_no: self.stage_no })
    }

    fn get_stage_no(&self) -> u16 { self.stage_no }
}

struct TrajAccessorImpl<'a> {
    formation: &'a Formation,
    pub stage_no: u16,
}
impl<'a> TrajAccessor for TrajAccessorImpl<'a> {
    fn get_formation_pos(&self, formation_index: &FormationIndex) -> Vec2I {
        self.formation.pos(formation_index)
    }
    fn get_stage_no(&self) -> u16 { self.stage_no }
}

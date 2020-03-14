use rand::Rng;

use crate::app::consts::*;
use crate::app::enemy::appearance_manager::AppearanceManager;
use crate::app::enemy::attack_manager::AttackManager;
use crate::app::enemy::enemy::{Enemy, EnemyState};
use crate::app::enemy::ene_shot::EneShot;
use crate::app::enemy::formation::Formation;
use crate::app::game::EventQueue;
use crate::app::util::{CollisionResult, CollBox, Collidable};
use crate::framework::RendererTrait;
use crate::framework::types::Vec2I;
use crate::util::math::ONE;

const MAX_ENEMY_COUNT: usize = 64;
const MAX_SHOT_COUNT: usize = 16;

pub struct EnemyManager {
    enemies: [Option<Enemy>; MAX_ENEMY_COUNT],
    shots: [Option<EneShot>; MAX_SHOT_COUNT],
    formation: Formation,
    appearance_manager: AppearanceManager,
    attack_manager: AttackManager,
}

impl EnemyManager {
    pub fn new() -> EnemyManager {
        let mut mgr = EnemyManager {
            enemies: [None; MAX_ENEMY_COUNT],
            shots: Default::default(),
            formation: Formation::new(),
            appearance_manager: AppearanceManager::new(0),
            attack_manager: AttackManager::new(),
        };
        mgr.restart();
        mgr
    }

    pub fn restart(&mut self) {
        for slot in self.enemies.iter_mut() {
            *slot = None;
        }
        for slot in self.shots.iter_mut() {
            *slot = None;
        }

        self.start_next_stage(0);
    }

    pub fn start_next_stage(&mut self, stage: u32) {
        self.appearance_manager = AppearanceManager::new(stage);
        self.formation.restart();
        self.attack_manager.restart();
    }

    pub fn all_destroyed(&self) -> bool {
        self.appearance_manager.done && self.enemies.iter().all(|x| x.is_none())
    }

    pub fn update(&mut self, event_queue: &mut EventQueue) {
        self.update_appearance();
        self.update_formation();
        self.update_attackers();
        self.update_enemies(event_queue);
        self.update_shots();
    }

    pub fn draw<R>(&self, renderer: &mut R) -> Result<(), String>
        where R: RendererTrait
    {
        for enemy in self.enemies.iter().flat_map(|x| x) {
            enemy.draw(renderer)?;
        }
        for shot in self.shots.iter().flat_map(|x| x) {
            shot.draw(renderer)?;
        }

        Ok(())
    }

    pub fn check_collision(&mut self, target: &CollBox, power: u32) -> CollisionResult {
        for enemy_opt in self.enemies.iter_mut().filter(|x| x.is_some()) {
            let enemy = enemy_opt.as_mut().unwrap();
            if enemy.get_collbox().check_collision(target) {
                let pos = enemy.pos();
                let destroyed = enemy.set_damage(power);
                if destroyed {
                    *enemy_opt = None;
                }
                return CollisionResult::Hit(pos, destroyed);
            }
        }

        return CollisionResult::NoHit;
    }

    pub fn check_shot_collision(&mut self, target: &CollBox) -> CollisionResult {
        for shot_opt in self.shots.iter_mut().filter(|x| x.is_some()) {
            let shot = shot_opt.as_mut().unwrap();
            if shot.get_collbox().check_collision(target) {
                let pos = shot.pos();
                *shot_opt = None;
                return CollisionResult::Hit(pos, false);
            }
        }

        return CollisionResult::NoHit;
    }

    fn update_appearance(&mut self) {
        let prev_done = self.appearance_manager.done;
        if let Some(new_borns) = self.appearance_manager.update() {
            for enemy in new_borns {
                self.spawn(enemy);
            }
        }
        if !prev_done && self.appearance_manager.done {
            self.formation.done_appearance();
        }
    }

    fn spawn(&mut self, enemy: Enemy) -> bool {
        if let Some(index) = self.find_slot() {
            self.enemies[index] = Some(enemy);
            true
        } else {
            false
        }
    }

    fn update_formation(&mut self) {
        let is_settle = self.formation.is_settle();
        self.formation.update();
        self.copy_formation_positions();

        if !is_settle && self.formation.is_settle() {
            self.attack_manager.set_enable(true);
        }
    }

    fn update_attackers(&mut self) {
        self.attack_manager.update(&mut self.enemies);
    }

    fn copy_formation_positions(&mut self) {
        for enemy in self.enemies.iter_mut().flat_map(|x| x).filter(|x| x.state == EnemyState::Formation) {
            let index = enemy.formation_index as usize;
            let pos = self.formation.pos(index & 15, index / 16);
            enemy.pos = pos;
        }
    }

    fn update_enemies(&mut self, event_queue: &mut EventQueue) {
        for enemy_opt in self.enemies.iter_mut().filter(|x| x.is_some()) {
            let enemy = enemy_opt.as_mut().unwrap();
            enemy.update(&self.formation, event_queue);
        }
    }

    fn update_shots(&mut self) {
        for shot_opt in self.shots.iter_mut().filter(|x| x.is_some()) {
            let shot = shot_opt.as_mut().unwrap();
            shot.update();
            if out_of_screen(shot.pos()) {
                *shot_opt = None;
            }
        }
    }

    fn find_slot(&self) -> Option<usize> {
        self.enemies.iter().position(|x| x.is_none())
    }

    pub fn spawn_shot(&mut self, pos: Vec2I, target_pos: &[Option<Vec2I>], speed: i32) {
        if let Some(index) = self.shots.iter().position(|x| x.is_none()) {
            let mut rng = rand::thread_rng();
            let count = target_pos.iter().filter(|x| x.is_some()).count();
            let target_opt: &Option<Vec2I> = target_pos.iter().filter(|x| x.is_some()).nth(rng.gen_range(0, count)).unwrap();
            let target: Vec2I = target_opt.unwrap();
            let d = target * ONE - pos;
            let distance = ((d.x as f64).powi(2) + (d.y as f64).powi(2)).sqrt();
            let f = (speed as f64) / distance;
            let vel = Vec2I::new(
                ((d.x as f64) * f).round() as i32,
                ((d.y as f64) * f).round() as i32,
            );
            self.shots[index] = Some(EneShot::new(
                pos, vel,
            ));
        }
    }
}

fn out_of_screen(pos: Vec2I) -> bool {
    pos.x < -16 || pos.x > WIDTH + 16
        || pos.y < -16 || pos.y > HEIGHT + 16
}

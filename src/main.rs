use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Player {
    Black,
    White,
}

impl Player {
    fn opponent(self) -> Self {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }

    fn as_char(&self) -> char {
        match self {
            Player::Black => '●', // 黑色实心圆
            Player::White => '○', // 白色空心圆
        }
    }

    fn color_name(&self) -> &str {
        match self {
            Player::Black => "黑方",
            Player::White => "白方",
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.color_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Empty,
    Occupied(Player),
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cell::Empty => write!(f, "·"), // 使用点表示空位
            Cell::Occupied(p) => write!(f, "{}", p.as_char()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum GamePhase {
    Placement, // 落子阶段
    Capture,   // 吃棋阶段
    Movement,  // 走子阶段
}

impl fmt::Display for GamePhase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GamePhase::Placement => write!(f, "落子阶段"),
            GamePhase::Capture => write!(f, "吃棋阶段"),
            GamePhase::Movement => write!(f, "走子阶段"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardPattern {
    Square { top_left: (usize, usize) }, // 成方
    Tri { id: usize },                   // 成三斜
    Tetra { id: usize },                 // 成四斜
    Row { index: usize },                // 成州(行)
    Col { index: usize },                // 成州(列)
    Dragon { id: usize },                // 成龙
}

impl fmt::Display for RewardPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewardPattern::Square { top_left: (r, c) } => write!(f, "成方[位置:({},{})]", r, c),
            RewardPattern::Tri { id } => write!(f, "成三斜[模式:{}]", id),
            RewardPattern::Tetra { id } => write!(f, "成四斜[模式:{}]", id),
            RewardPattern::Row { index } => write!(f, "成州[行:{}]", index),
            RewardPattern::Col { index } => write!(f, "成州[列:{}]", index),
            RewardPattern::Dragon { id } => write!(f, "成龙[对角线:{}]", id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameAction {
    Place {
        player: Player,
        pos: (usize, usize),
    },
    Capture {
        player: Player,
        pos: (usize, usize),
    },
    Move {
        player: Player,
        from: (usize, usize),
        to: (usize, usize),
    },
    Reward {
        player: Player,
        pattern: RewardPattern,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Board {
    grid: [[Cell; 5]; 5], // 5x5棋盘
    current_player: Player,
    phase: GamePhase,
    // 落子阶段专用
    extra_moves: u32, // 额外落子次数
    // 吃棋阶段专用
    capture_remaining: HashMap<Player, u32>, // 剩余吃子数量
    capture_turn: Player,                    // 当前吃棋玩家
    // 用于记录已触发的奖励模式
    triggered_squares: HashSet<[usize; 2]>, // 成方 [左上角行, 左上角列]
    triggered_tris: HashSet<usize>,         // 成三斜 0-3
    triggered_tetras: HashSet<usize>,       // 成四斜 0-3
    triggered_rows: HashSet<usize>,         // 成州行 0-4
    triggered_cols: HashSet<usize>,         // 成州列 0-4
    triggered_dragons: HashSet<usize>,      // 成龙 0-1
    // 奖励模式保护的棋子
    reward_pieces: HashMap<Player, HashSet<(usize, usize)>>,
    // 游戏记录
    game_record: Vec<GameAction>,
    movement_phase_origin: MovementPhaseOrigin, // 添加这个字段
}

// 添加枚举来标识进入移动阶段的方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum MovementPhaseOrigin {
    FromPlacement, // 从落子阶段进入（满盘后）
    FromCapture,   // 从吃棋阶段进入
    FromMovement,  // 从移动阶段自身进入（如吃棋后返回）
}


impl Board {
    pub fn new() -> Self {
        let mut board = Board {
            grid: [[Cell::Empty; 5]; 5],
            current_player: Player::Black,
            phase: GamePhase::Placement,
            extra_moves: 0,
            capture_remaining: HashMap::new(),
            capture_turn: Player::Black,
            triggered_squares: HashSet::new(),
            triggered_tris: HashSet::new(),
            triggered_tetras: HashSet::new(),
            triggered_rows: HashSet::new(),
            triggered_cols: HashSet::new(),
            triggered_dragons: HashSet::new(),
            reward_pieces: HashMap::new(),
            game_record: Vec::new(),
            movement_phase_origin: MovementPhaseOrigin::FromPlacement, // 默认从落子阶段进入
        };

        // 初始化奖励棋子保护集
        board.update_reward_pieces();
        board
    }

    pub fn admit_defeat(&self, remark : &String) -> bool {
        if remark.eq("f") {
            print!("玩家{}，认输！", self.current_player);
            return true; 
        }
        false
    }

    // 获取当前游戏状态
    pub fn get_state(&self) -> (GamePhase, Player) {
        (self.phase.clone(), self.current_player)
    }

    // 获取游戏记录
    pub fn get_game_record(&self) -> &Vec<GameAction> {
        &self.game_record
    }

    // // 打印棋盘
    // pub fn print_board(&self) {
    //     println!("  0 1 2 3 4");
    //     for (i, row) in self.grid.iter().enumerate() {
    //         print!("{} ", i);
    //         for cell in row {
    //             print!("{} ", cell);
    //         }
    //         println!();
    //     }
    // }

    // 打印棋盘（增强版）
    pub fn print_board(&self) {
        println!("\n  0 1 2 3 4  ← 列坐标");
        for (i, row) in self.grid.iter().enumerate() {
            print!("{} ", i); // 行坐标
            for cell in row {
                print!("{} ", cell);
            }
            println!();
        }
        println!("↑ 行坐标");
    }

    // 打印游戏状态
    // pub fn print_game_status(&self) {
    //     println!("\n===== 游戏状态 =====");
    //     println!("当前阶段: {}", self.phase);
    //     println!("当前玩家: {}", self.current_player);

    //     match self.phase {
    //         GamePhase::Placement => {
    //             if self.extra_moves > 0 {
    //                 println!("额外落子次数: {}", self.extra_moves);
    //             }
    //             println!("提示: 请输入落子位置 (格式: 行,列), 例如: 2,3");
    //         }
    //         GamePhase::Capture => {
    //             let remaining = self.capture_remaining.get(&self.current_player)
    //                 .copied().unwrap_or(0);
    //             println!("剩余吃子数量: {}", remaining);
    //             println!("提示: 请输入吃子位置 (格式: 行,列), 例如: 1,2");
    //         }
    //         GamePhase::Movement => {
    //             println!("提示: 请输入要移动的棋子位置和目标位置 (格式: 原行,原列 新行,新列), 例如: 1,2 1,3");
    //         }
    //     }
    // }

    pub fn print_game_status(&self) {
        println!("\n===== 游戏状态 =====");
        println!("当前阶段: {}", self.phase);
        println!("当前玩家: {}", self.current_player);

        match self.phase {
            GamePhase::Placement => {
                if self.extra_moves > 0 {
                    println!("额外落子次数: {}", self.extra_moves);
                }
                println!("提示: 请输入落子位置 (格式: 行,列), 例如: 2,3");
            }
            GamePhase::Capture => {
                let remaining = self
                    .capture_remaining
                    .get(&self.current_player)
                    .copied()
                    .unwrap_or(0);
                println!("剩余吃子数量: {}", remaining);
                println!("提示: 请输入吃子位置 (格式: 行,列), 例如: 1,2");
                println!("注意: 不能吃受保护棋子（在奖励模式中的棋子）");
            }
            GamePhase::Movement => {
                println!(
                    "提示: 请输入要移动的棋子位置和目标位置 (格式: 原行,原列 新行,新列), 例如: 1,2 1,3"
                );
                println!("注意: 只能移动到相邻位置（上下左右）");
            }
        }
    }

    // 记录游戏动作
    fn record_action(&mut self, action: GameAction) {
        self.game_record.push(action);
    }

    // 检查位置是否有效
    fn is_valid_pos(row: usize, col: usize) -> bool {
        row < 5 && col < 5
    }

    // 检查棋盘是否已满
    pub fn is_full(&self) -> bool {
        self.grid
            .iter()
            .all(|row| row.iter().all(|c| *c != Cell::Empty))
    }

    // 获取玩家棋子位置
    pub fn player_pieces(&self, player: Player) -> Vec<(usize, usize)> {
        let mut pieces = Vec::new();
        for (r, row) in self.grid.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if let Cell::Occupied(p) = cell {
                    if *p == player {
                        pieces.push((r, c));
                    }
                }
            }
        }
        pieces
    }

    // 检查玩家是否有合法移动
    pub fn has_legal_moves(&self, player: Player) -> bool {
        let pieces = self.player_pieces(player);

        // 如果棋子少于3个，无法形成任何模式，自动判负
        if pieces.len() < 3 {
            return false;
        }

        for (r, c) in pieces {
            // 检查上下左右四个方向
            let neighbors = [
                (r.wrapping_sub(1), c),
                (r + 1, c),
                (r, c.wrapping_sub(1)),
                (r, c + 1),
            ];

            for (nr, nc) in neighbors {
                if Self::is_valid_pos(nr, nc) && self.grid[nr][nc] == Cell::Empty {
                    return true;
                }
            }
        }

        false
    }

    // 执行落子
    pub fn place_piece(&mut self, row: usize, col: usize) -> Result<u32, &'static str> {
        if self.phase != GamePhase::Placement {
            return Err("当前不是落子阶段");
        }

        if !Self::is_valid_pos(row, col) {
            return Err("位置无效，必须在0-4范围内");
        }

        if self.grid[row][col] != Cell::Empty {
            return Err("该位置已有棋子，请选择空位");
        }

        // 落子
        self.grid[row][col] = Cell::Occupied(self.current_player);

        // 记录落子动作
        self.record_action(GameAction::Place {
            player: self.current_player,
            pos: (row, col),
        });

        // 检查奖励并获取额外落子次数
        let extra = self.check_rewards(row, col);

        // 处理额外落子次数
        self.extra_moves = self.extra_moves.saturating_add(extra);

        // 消耗一次落子机会
        if self.extra_moves > 0 {
            self.extra_moves -= 1;
        } else {
            self.current_player = self.current_player.opponent();
        }

        // 检查是否需要进入吃棋阶段
        if self.is_full() {
            self.enter_capture_phase();
        }

        Ok(extra)
    }

    // 进入吃棋阶段
    fn enter_capture_phase(&mut self) {
        self.phase = GamePhase::Capture;

     // 重置奖励模式记录并重新计算
    self.triggered_squares.clear();
    self.triggered_tris.clear();
    self.triggered_tetras.clear();
    self.triggered_rows.clear();
    self.triggered_cols.clear();
    self.triggered_dragons.clear();

    self.scan_all_rewards();


        // 设置吃棋顺序：第二个落子的玩家（白方）先吃棋
    let first_player = Player::White; // 白方是先吃玩家
    let second_player = Player::Black; // 黑方是后吃玩家


      // 重置吃子数量为0，与落子阶段无关
    self.capture_remaining = HashMap::new();
    self.capture_remaining.insert(first_player, 0);
    self.capture_remaining.insert(second_player, 0);

    // 检查是否有可吃的棋子
    let first_has_capturable = self.has_capturable_pieces(second_player);
    let second_has_capturable = self.has_capturable_pieces(first_player);


        // 只有当玩家有可吃棋子时才设置吃棋数量
    if first_has_capturable {
        self.capture_remaining.insert(first_player, 1);
    } else {
        self.capture_remaining.insert(first_player, 0);
    }

    if second_has_capturable {
        self.capture_remaining.insert(second_player, 1);
    } else {
        self.capture_remaining.insert(second_player, 0);
    }

         // 设置第一个有可吃棋子的玩家为当前玩家
    if first_has_capturable {
        self.current_player = first_player;
        self.capture_turn = first_player;
    } else if second_has_capturable {
        self.current_player = second_player;
        self.capture_turn = second_player;
    } else {
        // 如果都没有可吃的棋子，直接进入移动阶段
        self.enter_movement_phase(MovementPhaseOrigin::FromPlacement);
        return;
    }
    }

    // 检查是否有可吃的棋子
    fn has_capturable_pieces(&self, opponent: Player) -> bool {
        let protected = self
            .reward_pieces
            .get(&opponent)
            .cloned()
            .unwrap_or_default();

        // 检查对手的所有棋子
        for r in 0..5 {
            for c in 0..5 {
                if let Cell::Occupied(p) = self.grid[r][c] {
                    if p == opponent && !protected.contains(&(r, c)) {
                        return true;
                    }
                }
            }
        }
        false
    }

    // 扫描所有奖励模式
    fn scan_all_rewards(&mut self) {
        for player in [Player::Black, Player::White] {
            // 扫描成方
            for r in 0..4 {
                for c in 0..4 {
                    if self.is_square(r, c, player) {
                        self.triggered_squares.insert([r, c]);
                    }
                }
            }

            // 扫描成三斜
            for id in 0..4 {
                if self.is_tri(id, player) {
                    self.triggered_tris.insert(id);
                }
            }

            // 扫描成四斜
            for id in 0..4 {
                if self.is_tetra(id, player) {
                    self.triggered_tetras.insert(id);
                }
            }

            // 扫描成州（行）
            for r in 0..5 {
                if self.is_row(r, player) {
                    self.triggered_rows.insert(r);
                }
            }

            // 扫描成州（列）
            for c in 0..5 {
                if self.is_col(c, player) {
                    self.triggered_cols.insert(c);
                }
            }

            // 扫描成龙
            for id in 0..2 {
                if self.is_dragon(id, player) {
                    self.triggered_dragons.insert(id);
                }
            }
        }

        // 更新保护棋子
        self.update_reward_pieces();
    }

    // 计算玩家吃子数量
    fn calculate_capture_count(&self, player: Player) -> u32 {
        let mut count = 0;

        // 成方数量
        for square in &self.triggered_squares {
            if self.is_square(square[0], square[1], player) {
                count += 1;
            }
        }

        // 成三斜数量
        for id in &self.triggered_tris {
            if self.is_tri(*id, player) {
                count += 1;
            }
        }

        // 成四斜数量
        for id in &self.triggered_tetras {
            if self.is_tetra(*id, player) {
                count += 1;
            }
        }

        // 成州数量
        count += self
            .triggered_rows
            .iter()
            .filter(|&&r| self.is_row(r, player))
            .count() as u32
            * 2;

        count += self
            .triggered_cols
            .iter()
            .filter(|&&c| self.is_col(c, player))
            .count() as u32
            * 2;

        // 成龙数量
        count += self
            .triggered_dragons
            .iter()
            .filter(|&&id| self.is_dragon(id, player))
            .count() as u32
            * 2;

        count
    }

    // 更新受保护的棋子
    fn update_reward_pieces(&mut self) {
        self.reward_pieces.clear();

        // 黑方受保护棋子
        let mut black_protected = HashSet::new();
        self.add_reward_pieces(Player::Black, &mut black_protected);
        self.reward_pieces.insert(Player::Black, black_protected);

        // 白方受保护棋子
        let mut white_protected = HashSet::new();
        self.add_reward_pieces(Player::White, &mut white_protected);
        self.reward_pieces.insert(Player::White, white_protected);
    }

    // 收集奖励模式中的棋子
    fn add_reward_pieces(&self, player: Player, protected: &mut HashSet<(usize, usize)>) {
        // 成方
        for &[r, c] in &self.triggered_squares {
            if self.is_square(r, c, player) {
                protected.insert((r, c));
                protected.insert((r, c + 1));
                protected.insert((r + 1, c));
                protected.insert((r + 1, c + 1));
            }
        }

        // 成三斜
        let tris = [
            vec![(0, 2), (1, 1), (2, 0)], // 左上三斜
            vec![(0, 2), (1, 3), (2, 4)], // 右上三斜
            vec![(2, 0), (3, 1), (4, 2)], // 左下三斜
            vec![(2, 4), (3, 3), (4, 2)], // 右下三斜
        ];
        for id in &self.triggered_tris {
            if let Some(tri) = tris.get(*id) {
                if self.is_tri(*id, player) {
                    for &(r, c) in tri {
                        protected.insert((r, c));
                    }
                }
            }
        }

        // 成四斜
        let tetras = [
            vec![(0, 1), (1, 2), (2, 3), (3, 4)], // 左上四斜
            vec![(0, 3), (1, 2), (2, 1), (3, 0)], // 右上四斜
            vec![(1, 0), (2, 1), (3, 2), (4, 3)], // 左下四斜
            vec![(1, 4), (2, 3), (3, 2), (4, 1)], // 右下四斜
        ];
        for id in &self.triggered_tetras {
            if let Some(tetra) = tetras.get(*id) {
                if self.is_tetra(*id, player) {
                    for &(r, c) in tetra {
                        protected.insert((r, c));
                    }
                }
            }
        }

        // 成州（行）
        for &r in &self.triggered_rows {
            if self.is_row(r, player) {
                for c in 0..5 {
                    protected.insert((r, c));
                }
            }
        }

        // 成州（列）
        for &c in &self.triggered_cols {
            if self.is_col(c, player) {
                for r in 0..5 {
                    protected.insert((r, c));
                }
            }
        }

        // 成龙
        let dragons = [
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)], // 主对角线
            vec![(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)], // 副对角线
        ];
        for id in &self.triggered_dragons {
            if let Some(dragon) = dragons.get(*id) {
                if self.is_dragon(*id, player) {
                    for &(r, c) in dragon {
                        protected.insert((r, c));
                    }
                }
            }
        }
    }

    // 执行吃棋（单步吃一个棋子）
    pub fn capture_piece(&mut self, row: usize, col: usize) -> Result<(), &'static str> {
        if self.phase != GamePhase::Capture {
            return Err("当前不是吃棋阶段");
        }

        let player = self.current_player;

        // 获取当前玩家剩余吃子数量
        let remaining = match self.capture_remaining.get(&player) {
            Some(&r) if r > 0 => r,
            _ => return Err("没有待处理的吃棋任务"),
        };

        if !Self::is_valid_pos(row, col) {
            return Err("位置无效");
        }

        let opponent = player.opponent();
        let protected = self
            .reward_pieces
            .get(&opponent)
            .cloned()
            .unwrap_or_default();

        // 验证吃棋位置
        if protected.contains(&(row, col)) {
            return Err("不能吃受保护的棋子");
        }

        if let Cell::Occupied(p) = self.grid[row][col] {
            if p != opponent {
                return Err("只能吃对方棋子");
            }
        } else {
            return Err("该位置没有棋子");
        }

        // 执行吃棋
        self.grid[row][col] = Cell::Empty;

        // 记录吃棋动作
        self.record_action(GameAction::Capture {
            player,
            pos: (row, col),
        });

        // 更新吃棋剩余数量
        *self.capture_remaining.get_mut(&player).unwrap() = remaining - 1;

        

        // 更新奖励棋子保护集
        self.update_reward_pieces();

        // 检查吃棋后状态
        if self.capture_remaining.values().sum::<u32>() == 0 {
    // 所有吃棋完成，进入移动阶段
    self.enter_movement_phase(MovementPhaseOrigin::FromMovement);
    return Ok(());
        }

           // 如果当前玩家还有吃子机会，不切换玩家
    if self.capture_remaining.get(&player).copied().unwrap_or(0) > 0 {
        return Ok(());
    }

        // 切换到下一个吃棋玩家
        let next_player = player.opponent();

        // 如果下一个玩家没有可吃的棋子，则跳过
// 检查下一个玩家是否有可吃的棋子
let next_has_capturable = self.has_capturable_pieces(next_player.opponent());

if !next_has_capturable {
    // 如果下一个玩家没有可吃的棋子，检查是否所有玩家都完成吃棋
    if self.capture_remaining.values().sum::<u32>() == 0 {
        self.enter_movement_phase(MovementPhaseOrigin::FromCapture);
        return Ok(());
    } else {
        // 跳过这个玩家，回到第一个吃棋玩家
        self.current_player = self.capture_turn;
        return Ok(());
    }
}

self.current_player = next_player;

        // 检查后吃棋责任
        if player == self.capture_turn.opponent() {
            let next_mover = self.capture_turn;
            if !self.has_legal_moves(next_mover) {
                return Err("后吃棋导致对方无法走棋，你输了");
            }
        }

        Ok(())
    }

    // 进入移动阶段
fn enter_movement_phase(&mut self, origin: MovementPhaseOrigin) {
    self.phase = GamePhase::Movement;
    self.movement_phase_origin = origin;
    
    match origin {
        MovementPhaseOrigin::FromPlacement => {
            // 从满盘进入移动阶段，白方先走
            self.current_player = Player::White;
        }
        MovementPhaseOrigin::FromCapture => {
            // 从吃棋阶段进入移动阶段，保持当前玩家不变
            // 不需要改变current_player
        }
        MovementPhaseOrigin::FromMovement => {
            // 从移动阶段自身进入（如吃棋后返回），切换玩家
            self.current_player = self.current_player.opponent();
        }
    }
    
    self.update_reward_pieces();
}

// 添加新的奖励检查方法，专门用于走棋阶段
fn check_rewards_after_move(&mut self, row: usize, col: usize) -> u32 {
    let player = self.current_player;
    let mut capture_count = 0;
    
    // 只检查与移动棋子相关的奖励模式
    // 1. 检查成方 (1x1 正方形)
    let squares = self.check_squares_after_move(row, col, player);
    capture_count += squares;
    
    // 2. 检查成三斜 (3点斜线)
    let tris = self.check_tris_after_move(row, col, player);
    capture_count += tris;
    
    // 3. 检查成四斜 (4点斜线)
    let tetras = self.check_tetras_after_move(row, col, player);
    capture_count += tetras;
    
    // 4. 检查成州 (整行或整列)
    let rows = self.check_rows_after_move(row, player);
    capture_count += rows;
    
    let cols = self.check_cols_after_move(col, player);
    capture_count += cols;
    
    // 5. 检查成龙 (对角线)
    let dragons = self.check_dragons_after_move(row, col, player);
    capture_count += dragons;
    
    capture_count
}

// 添加走棋阶段专用的奖励检查方法
fn check_squares_after_move(&mut self, row: usize, col: usize, player: Player) -> u32 {
    let mut extra = 0;
    
    // 检查可能包含该点的所有正方形
    for &(r, c) in &[
        (row, col),
        (row, col.saturating_sub(1)),
        (row.saturating_sub(1), col),
        (row.saturating_sub(1), col.saturating_sub(1)),
    ] {
        if r < 4 && c < 4 {
            if self.is_square(r, c, player) && !self.triggered_squares.contains(&[r, c]) {
                self.triggered_squares.insert([r, c]);
                extra += 1;
                
                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Square { top_left: (r, c) },
                });
            }
        }
    }
    extra
}

fn check_tris_after_move(&mut self, row: usize, col: usize, player: Player) -> u32 {
    let mut extra = 0;
    
    for id in 0..4 {
        if self.is_tri_affected_by_move(id, row, col, player) && 
           self.is_tri(id, player) && 
           !self.triggered_tris.contains(&id) {
            self.triggered_tris.insert(id);
            extra += 1;
            
            // 记录奖励模式
            self.record_action(GameAction::Reward {
                player,
                pattern: RewardPattern::Tri { id },
            });
        }
    }
    extra
}

fn check_tetras_after_move(&mut self, row: usize, col: usize, player: Player) -> u32 {
    let mut extra = 0;
    
    for id in 0..4 {
        if self.is_tetra_affected_by_move(id, row, col, player) && 
           self.is_tetra(id, player) && 
           !self.triggered_tetras.contains(&id) {
            self.triggered_tetras.insert(id);
            extra += 1;
            
            // 记录奖励模式
            self.record_action(GameAction::Reward {
                player,
                pattern: RewardPattern::Tetra { id },
            });
        }
    }
    extra
}

fn check_rows_after_move(&mut self, row: usize, player: Player) -> u32 {
    if self.is_row(row, player) && !self.triggered_rows.contains(&row) {
        self.triggered_rows.insert(row);
        
        // 记录奖励模式
        self.record_action(GameAction::Reward {
            player,
            pattern: RewardPattern::Row { index: row },
        });
        
        2 // 成州奖励2次吃子机会
    } else {
        0
    }
}

fn check_cols_after_move(&mut self, col: usize, player: Player) -> u32 {
    if self.is_col(col, player) && !self.triggered_cols.contains(&col) {
        self.triggered_cols.insert(col);
        
        // 记录奖励模式
        self.record_action(GameAction::Reward {
            player,
            pattern: RewardPattern::Col { index: col },
        });
        
        2 // 成州奖励2次吃子机会
    } else {
        0
    }
}

fn check_dragons_after_move(&mut self, row: usize, col: usize, player: Player) -> u32 {
    let mut extra = 0;
    
    for id in 0..2 {
        if self.is_dragon_affected_by_move(id, row, col, player) && 
           self.is_dragon(id, player) && 
           !self.triggered_dragons.contains(&id) {
            self.triggered_dragons.insert(id);
            extra += 2; // 成龙奖励2次吃子机会
            
            // 记录奖励模式
            self.record_action(GameAction::Reward {
                player,
                pattern: RewardPattern::Dragon { id },
            });
        }
    }
    extra
}

// 添加辅助方法检查移动是否影响特定模式
fn is_tri_affected_by_move(&self, id: usize, row: usize, col: usize, player: Player) -> bool {
    let positions = match id {
        0 => vec![(0, 2), (1, 1), (2, 0)], // 左上三斜
        1 => vec![(0, 2), (1, 3), (2, 4)], // 右上三斜
        2 => vec![(2, 0), (3, 1), (4, 2)], // 左下三斜
        3 => vec![(2, 4), (3, 3), (4, 2)], // 右下三斜
        _ => return false,
    };
    
    positions.contains(&(row, col))
}

fn is_tetra_affected_by_move(&self, id: usize, row: usize, col: usize, player: Player) -> bool {
    let positions = match id {
        0 => vec![(0, 1), (1, 2), (2, 3), (3, 4)], // 左上四斜
        1 => vec![(0, 3), (1, 2), (2, 1), (3, 0)], // 右上四斜
        2 => vec![(1, 0), (2, 1), (3, 2), (4, 3)], // 左下四斜
        3 => vec![(1, 4), (2, 3), (3, 2), (4, 1)], // 右下四斜
        _ => return false,
    };
    
    positions.contains(&(row, col))
}

fn is_dragon_affected_by_move(&self, id: usize, row: usize, col: usize, player: Player) -> bool {
    let positions = match id {
        0 => vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)], // 主对角线
        1 => vec![(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)], // 副对角线
        _ => return false,
    };
    
    positions.contains(&(row, col))
}

    // 执行移动
    pub fn move_piece(
        &mut self,
        from: (usize, usize),
        to: (usize, usize),
    ) -> Result<u32, &'static str> {
        if self.phase != GamePhase::Movement {
            return Err("当前不是走子阶段");
        }

        let (from_row, from_col) = from;
        let (to_row, to_col) = to;

        // 验证移动位置
        if !Self::is_valid_pos(from_row, from_col) || !Self::is_valid_pos(to_row, to_col) {
            return Err("位置无效，必须在0-4范围内");
        }

        // 检查起始位置是否属于当前玩家
        if let Cell::Occupied(p) = self.grid[from_row][from_col] {
            if p != self.current_player {
                return Err("只能移动自己的棋子");
            }
        } else {
            return Err("起始位置无棋子");
        }

        // 检查目标位置是否为空
        if self.grid[to_row][to_col] != Cell::Empty {
            return Err("目标位置已被占用");
        }

        // 检查移动是否相邻（上下左右）
        let row_diff = from_row.abs_diff(to_row);
        let col_diff = from_col.abs_diff(to_col);
        if (row_diff == 1 && col_diff == 0) || (row_diff == 0 && col_diff == 1) {
            // 有效移动
        } else {
            return Err("只能移动到相邻位置（上下左右）");
        }

        // 执行移动
        let player = self.current_player;
        self.grid[from_row][from_col] = Cell::Empty;
        self.grid[to_row][to_col] = Cell::Occupied(player);

        // 记录移动动作
        self.record_action(GameAction::Move {
            player,
            from: (from_row, from_col),
            to: (to_row, to_col),
        });

        // 检查奖励并获取可吃子数量
       let capture_count = self.check_rewards_after_move(to_row, to_col);


       // 如果有吃子机会，进入吃棋阶段让玩家选择吃哪些棋子
    if capture_count > 0 {
        // 设置吃棋阶段
        self.phase = GamePhase::Capture;
        self.capture_remaining.insert(player, capture_count);
        self.capture_turn = player;
        
        // 更新奖励棋子保护集
        self.update_reward_pieces();
        
        // 返回吃子数量，但不实际吃子
        return Ok(capture_count);
    }

        // 检查移动后对方是否能走棋
        let opponent = player.opponent();
        if !self.has_legal_moves(opponent) {
            // 导致对方无法走棋，当前玩家判负
            return Err("移动导致对方无法走棋，你输了");
        }

        // 切换玩家
        self.current_player = self.current_player.opponent();

        Ok(0)
    }

    // 落子阶段的奖励检查
    fn check_rewards(&mut self, row: usize, col: usize) -> u32 {
        let mut extra = 0;
        let player = self.current_player;

        // 1. 检查成方 (1x1 正方形)
        let squares = self.check_squares(row, col);
        if squares > 0 {
            println!("玩家 {} 形成成方奖励", player);
        }
        extra += squares;

        // 2. 检查成三斜 (3点斜线)
        let tris = self.check_tris();
        if tris > 0 {
            println!("玩家 {} 形成成三斜奖励", player);
        }
        extra += tris;

        // 3. 检查成四斜 (4点斜线)
        let tetras = self.check_tetras();
        if tetras > 0 {
            println!("玩家 {} 形成成四斜奖励", player);
        }
        extra += tetras;

        // 4. 检查成州 (整行或整列)
        let rows = self.check_rows();
        if rows > 0 {
            println!("玩家 {} 形成成州(行)奖励", player);
        }
        extra += rows;

        let cols = self.check_cols();
        if cols > 0 {
            println!("玩家 {} 形成成州(列)奖励", player);
        }
        extra += cols;

        // 5. 检查成龙 (对角线)
        let dragons = self.check_dragons();
        if dragons > 0 {
            println!("玩家 {} 形成成龙奖励", player);
        }
        extra += dragons;

        extra
    }

    // 移动阶段的奖励检查
    fn check_rewards_movement(&mut self, row: usize, col: usize) -> u32 {
        // 重置所有奖励模式（因为移动可能改变模式）
        self.triggered_squares.clear();
        self.triggered_tris.clear();
        self.triggered_tetras.clear();
        self.triggered_rows.clear();
        self.triggered_cols.clear();
        self.triggered_dragons.clear();

        // 重新检查所有奖励模式
        let mut capture_count = 0;
        let player = self.current_player;

        // 1. 检查所有成方
        for r in 0..4 {
            for c in 0..4 {
                if self.is_square(r, c, player) {
                    self.triggered_squares.insert([r, c]);
                    capture_count += 1;

                    // 记录奖励模式
                    self.record_action(GameAction::Reward {
                        player,
                        pattern: RewardPattern::Square { top_left: (r, c) },
                    });
                    // println!("玩家 {} 形成成方[位置:({},{})]", player, r, c);
                }
            }
        }

        // 2. 检查所有成三斜
        for id in 0..4 {
            if self.is_tri(id, player) {
                self.triggered_tris.insert(id);
                capture_count += 1;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Tri { id },
                });
                // println!("玩家 {} 形成成三斜[模式:{}]", player, id);
            }
        }

        // 3. 检查所有成四斜
        for id in 0..4 {
            if self.is_tetra(id, player) {
                self.triggered_tetras.insert(id);
                capture_count += 1;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Tetra { id },
                });
                // println!("玩家 {} 形成成四斜[模式:{}]", player, id);
            }
        }

        // 4. 检查所有成州（行）
        for r in 0..5 {
            if self.is_row(r, player) {
                self.triggered_rows.insert(r);
                capture_count += 2;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Row { index: r },
                });
                // println!("玩家 {} 形成成州[行:{}]", player, r);
            }
        }

        // 4. 检查所有成州（列）
        for c in 0..5 {
            if self.is_col(c, player) {
                self.triggered_cols.insert(c);
                capture_count += 2;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Col { index: c },
                });
                // println!("玩家 {} 形成成州[列:{}]", player, c);
            }
        }

        // 5. 检查所有成龙
        for id in 0..2 {
            if self.is_dragon(id, player) {
                self.triggered_dragons.insert(id);
                capture_count += 2;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Dragon { id },
                });
                // println!("玩家 {} 形成成龙[对角线:{}]", player, id);
            }
        }

        capture_count
    }

    // 成方检测 (1x1 正方形)
    fn check_squares(&mut self, row: usize, col: usize) -> u32 {
        let mut extra = 0;
        let player = self.current_player;

        // 检查可能包含该点的所有正方形
        for &(r, c) in &[
            (row, col),
            (row, col.saturating_sub(1)),
            (row.saturating_sub(1), col),
            (row.saturating_sub(1), col.saturating_sub(1)),
        ] {
            if r < 4 && c < 4 {
                if self.is_square(r, c, player) {
                    let square_id = [r, c];
                    if self.triggered_squares.insert(square_id) {
                        extra += 1;

                        // 记录奖励模式
                        self.record_action(GameAction::Reward {
                            player,
                            pattern: RewardPattern::Square { top_left: (r, c) },
                        });
                    }
                }
            }
        }
        extra
    }

    fn is_square(&self, r: usize, c: usize, player: Player) -> bool {
        let corners = [(r, c), (r, c + 1), (r + 1, c), (r + 1, c + 1)];

        corners
            .iter()
            .all(|&(r, c)| matches!(self.grid[r][c], Cell::Occupied(p) if p == player))
    }

    // 成三斜检测 (3点斜线)
    fn check_tris(&mut self) -> u32 {
        let player = self.current_player;
        let mut extra = 0;

        for id in 0..4 {
            if !self.triggered_tris.contains(&id) && self.is_tri(id, player) {
                self.triggered_tris.insert(id);
                extra += 1;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Tri { id },
                });
            }
        }
        extra
    }

    fn is_tri(&self, id: usize, player: Player) -> bool {
        let positions = match id {
            0 => vec![(0, 2), (1, 1), (2, 0)], // 左上三斜
            1 => vec![(0, 2), (1, 3), (2, 4)], // 右上三斜
            2 => vec![(2, 0), (3, 1), (4, 2)], // 左下三斜
            3 => vec![(2, 4), (3, 3), (4, 2)], // 右下三斜
            _ => return false,
        };

        positions
            .iter()
            .all(|&(r, c)| matches!(self.grid[r][c], Cell::Occupied(p) if p == player))
    }

    // 成四斜检测 (4点斜线)
    fn check_tetras(&mut self) -> u32 {
        let player = self.current_player;
        let mut extra = 0;

        for id in 0..4 {
            if !self.triggered_tetras.contains(&id) && self.is_tetra(id, player) {
                self.triggered_tetras.insert(id);
                extra += 1;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Tetra { id },
                });
            }
        }
        extra
    }

    fn is_tetra(&self, id: usize, player: Player) -> bool {
        let positions = match id {
            0 => vec![(0, 1), (1, 2), (2, 3), (3, 4)], // 左上四斜
            1 => vec![(0, 3), (1, 2), (2, 1), (3, 0)], // 右上四斜
            2 => vec![(1, 0), (2, 1), (3, 2), (4, 3)], // 左下四斜
            3 => vec![(1, 4), (2, 3), (3, 2), (4, 1)], // 右下四斜
            _ => return false,
        };

        positions
            .iter()
            .all(|&(r, c)| matches!(self.grid[r][c], Cell::Occupied(p) if p == player))
    }

    // 成州检测 (整行)
    fn check_rows(&mut self) -> u32 {
        let player = self.current_player;
        let mut extra = 0;

        for r in 0..5 {
            if !self.triggered_rows.contains(&r) && self.is_row(r, player) {
                self.triggered_rows.insert(r);
                extra += 2;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Row { index: r },
                });
            }
        }
        extra
    }

    fn is_row(&self, r: usize, player: Player) -> bool {
        (0..5).all(|c| matches!(self.grid[r][c], Cell::Occupied(p) if p == player))
    }

    // 成州检测 (整列)
    fn check_cols(&mut self) -> u32 {
        let player = self.current_player;
        let mut extra = 0;

        for c in 0..5 {
            if !self.triggered_cols.contains(&c) && self.is_col(c, player) {
                self.triggered_cols.insert(c);
                extra += 2;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Col { index: c },
                });
            }
        }
        extra
    }

    fn is_col(&self, c: usize, player: Player) -> bool {
        (0..5).all(|r| matches!(self.grid[r][c], Cell::Occupied(p) if p == player))
    }

    // 成龙检测 (对角线)
    fn check_dragons(&mut self) -> u32 {
        let player = self.current_player;
        let mut extra = 0;

        for id in 0..2 {
            if !self.triggered_dragons.contains(&id) && self.is_dragon(id, player) {
                self.triggered_dragons.insert(id);
                extra += 2;

                // 记录奖励模式
                self.record_action(GameAction::Reward {
                    player,
                    pattern: RewardPattern::Dragon { id },
                });
            }
        }
        extra
    }

    fn is_dragon(&self, id: usize, player: Player) -> bool {
        let positions = match id {
            0 => vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)], // 主对角线
            1 => vec![(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)], // 副对角线
            _ => return false,
        };

        positions
            .iter()
            .all(|&(r, c)| matches!(self.grid[r][c], Cell::Occupied(p) if p == player))
    }

    // 检查游戏是否结束
    pub fn check_winner(&self) -> Option<Player> {
        // 只在吃棋和走子阶段检查
        if self.phase == GamePhase::Placement {
            return None;
        }

        let black_pieces = self.player_pieces(Player::Black).len();
        let white_pieces = self.player_pieces(Player::White).len();

        if black_pieces < 3 {
            return Some(Player::White);
        }

        if white_pieces < 3 {
            return Some(Player::Black);
        }

        // 检查是否有合法移动
        if self.phase == GamePhase::Movement {
            if !self.has_legal_moves(self.current_player) {
                return Some(self.current_player.opponent());
            }
        }

        None
    }
}

// 棋谱重放器
pub struct GameReplayer {
    actions: Vec<GameAction>,
    current_step: usize,
    board: Board,
}

impl GameReplayer {
    pub fn new(actions: Vec<GameAction>) -> Self {
        GameReplayer {
            actions,
            current_step: 0,
            board: Board::new(),
        }
    }

    pub fn step_forward(&mut self) -> Option<&Board> {
        if self.current_step >= self.actions.len() {
            return None;
        }

        let action = &self.actions[self.current_step];
        match action {
            GameAction::Place { player, pos } => {
                self.board.place_piece(pos.0, pos.1).ok();
            }
            GameAction::Capture { player, pos } => {
                self.board.capture_piece(pos.0, pos.1).ok();
            }
            GameAction::Move { player, from, to } => {
                self.board.move_piece(*from, *to).ok();
            }
            _ => {} // 奖励模式不需要执行操作
        }

        self.current_step += 1;
        Some(&self.board)
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
        self.board = Board::new();
    }

    pub fn get_current_board(&self) -> &Board {
        &self.board
    }
}

// 读取用户输入
fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

// 解析坐标输入
fn parse_coord(input: &str) -> Result<(usize, usize), &'static str> {
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 2 {
        return Err("输入格式错误，请使用 行,列 格式，例如: 2,3");
    }

    let row = parts[0]
        .parse::<usize>()
        .map_err(|_| "行号必须是0-4之间的数字")?;
    let col = parts[1]
        .parse::<usize>()
        .map_err(|_| "列号必须是0-4之间的数字")?;

    if row > 4 || col > 4 {
        return Err("行和列必须在0-4范围内");
    }

    Ok((row, col))
}

fn parse_move(input: &str) -> Result<((usize, usize), (usize, usize)), &'static str> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 2 {
        return Err("输入格式错误，请使用 原行,原列 新行,新列 格式");
    }

    let from = parse_coord(parts[0])?;
    let to = parse_coord(parts[1])?;

    Ok((from, to))
}

// 主游戏循环
// fn main() {
//     println!("\n===== 欢迎来到五道方游戏! =====");
//     println!("游戏规则说明:");
//     println!("1. 游戏分为三个阶段: 落子阶段、吃棋阶段、走子阶段");
//     println!("2. 落子阶段: 玩家轮流在5x5棋盘上放置棋子");
//     println!(
//         "3. 形成特定模式可获得奖励: 成方(+1子)、成三斜(+1子)、成四斜(+1子)、成州(+2子)、成龙(+2子)"
//     );
//     println!("4. 棋盘满后进入吃棋阶段: 后落子的玩家先吃棋，轮流吃掉对方棋子");
//     println!("5. 吃棋完成后进入走子阶段: 玩家轮流移动自己的棋子");
//     println!("6. 胜利条件: 对方棋子少于3个或无法移动时获胜");
//     println!("================================\n");

//     let mut board = Board::new();
//     let mut game_over = false;

//     while !game_over {
//         board.print_board();
//         board.print_game_status();

//         // 检查胜利条件（只在吃棋和走子阶段）
//         if let Some(winner) = board.check_winner() {
//             println!("\n===== 游戏结束! =====");
//             println!("{} 获胜!", winner);
//             game_over = true;
//             continue;
//         }

//         match board.phase {
//             GamePhase::Placement => {
//                 let input = read_input(&format!("{} 请输入落子位置: ", board.current_player));

//                 if board.admit_defeat(&input) {
//                     println!("{} 认输，游戏结束！", board.current_player);
//                     game_over = true;
//                     continue;
//                 }
//                 match parse_coord(&input) {
//                     Ok((row, col)) => match board.place_piece(row, col) {
//                         Ok(extra) => {
//                             if extra > 0 {
//                                 println!(
//                                     "{} 形成奖励模式，获得额外落子次数: {}",
//                                     board.current_player, extra
//                                 );
//                             }
//                         }
//                         Err(e) => println!("操作失败: {}", e),
//                     },
//                     Err(e) => println!("输入错误: {}", e),
//                 }
//             }

//             GamePhase::Capture => {
//                 // 检查当前玩家是否可以吃子
//                 let opponent = board.current_player.opponent();
//                 let opponent_pieces = board.player_pieces(opponent);
//                 let protected = board
//                     .reward_pieces
//                     .get(&opponent)
//                     .cloned()
//                     .unwrap_or_default();

//                 // 找出可吃的棋子（不在保护集中的）
//                 let capturable: Vec<_> = opponent_pieces
//                     .iter()
//                     .filter(|pos| !protected.contains(pos))
//                     .collect();

//                 // 如果当前玩家有吃子任务但无棋子可吃，自动跳过
//                 if capturable.is_empty() {
//                     let player = board.current_player;
//                     let remaining = board.capture_remaining.get(&player).copied().unwrap_or(0);

//                     if remaining > 0 {
//                         println!("{} 没有可吃的棋子，自动跳过吃棋阶段", player);

//                         // 清零当前玩家的吃子任务
//                         *board.capture_remaining.get_mut(&player).unwrap() = 0;

//                         // 检查是否所有玩家都完成吃棋
//                         if board.capture_remaining.values().sum::<u32>() == 0 {
//                             board.enter_movement_phase();
//                             continue;
//                         }

//                         // 切换到下一个玩家
//                         board.current_player = player.opponent();
//                         continue;
//                     }
//                 }

//                 // 正常处理吃棋输入
//                 let input = read_input(&format!("{} 请输入吃子位置: ", board.current_player));

//                 match parse_coord(&input) {
//                     Ok((row, col)) => match board.capture_piece(row, col) {
//                         Ok(_) => println!("吃棋成功!"),
//                         Err(e) => println!("操作失败: {}", e),
//                     },
//                     Err(e) => println!("输入错误: {}", e),
//                 }
//             }

//             GamePhase::Movement => {
//                 let input = read_input(&format!(
//                     "{} 请输入移动指令 (原位置 目标位置): ",
//                     board.current_player
//                 ));

//                 match parse_move(&input) {
//                     Ok((from, to)) => match board.move_piece(from, to) {
//                         Ok(captured) => {
//                             if captured > 0 {
//                                 println!("移动成功! 吃掉对方 {} 个棋子", captured);
//                             } else {
//                                 println!("移动成功!");
//                             }
//                         }
//                         Err(e) => println!("操作失败: {}", e),
//                     },
//                     Err(e) => println!("输入错误: {}", e),
//                 }
//             }
//         }
//     }

//     // 保存棋谱选项
//     let save = read_input("是否保存棋谱? (y/n): ");
//     if save.to_lowercase() == "y" {
//         let serialized = serde_json::to_string_pretty(board.get_game_record()).unwrap();
//         let filename = "wudao_game_record.json";
//         std::fs::write(filename, &serialized).unwrap();
//         println!("棋谱已保存到 {}", filename);
//     }

//     println!("\n感谢游玩五道方游戏！");
// }


use eframe::egui::{self, ViewportBuilder};
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use eframe::egui::{Color32, Stroke, FontId, Align2, RichText};
use std::f32::consts::PI;
fn main() -> eframe::Result<()> {

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
eframe::run_native(
        "五道方游戏",
        options,
        Box::new(|cc| {
            // 设置中文字体
            let mut fonts = FontDefinitions::default();
            
            // 使用系统字体或回退字体
            fonts.font_data.insert(
                "chinese".to_owned(),
                FontData::from_static(include_bytes!("../assets/fonts/NotoSansSC.ttf")), // 替换为您的中文字体路径
            );
            
            // 或者使用默认字体并添加中文字符支持
            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, "chinese".to_owned());
                
            fonts
                .families
                .get_mut(&FontFamily::Monospace)
                .unwrap()
                .push("chinese".to_owned());
                
            cc.egui_ctx.set_fonts(fonts);
            
            Box::new(WudaoApp::new())
        }),
    )
}

struct WudaoApp {
    board: Board,
    selected_cell: Option<(usize, usize)>,
    message: String,
    game_over: bool,
    show_help: bool,
    input_mode: InputMode,
    time: f32, // 用于动画效果的时间变量
}

#[derive(PartialEq)]
enum InputMode {
    Placement,
    Capture,
    MovementFrom,
    MovementTo,
}

impl WudaoApp {
    fn new() -> Self {
        Self {
            board: Board::new(),
            selected_cell: None,
            message: String::new(),
            game_over: false,
            show_help: true,
            input_mode: InputMode::Placement,
            time: 0.0,
        }
    }
    
    fn handle_cell_click(&mut self, row: usize, col: usize) {
    let (phase, player) = self.board.get_state();
    
    match phase {
        GamePhase::Placement => {
            match self.board.place_piece(row, col) {
                Ok(extra) => {
                    self.message = format!("在({},{})落子", row, col);
                    if extra > 0 {
                        self.message += &format!("，获得额外落子次数: {}", extra);
                    }
                }
                Err(e) => {
                    self.message = format!("落子失败: {}", e);
                }
            }
        }
        GamePhase::Capture => {
            match self.board.capture_piece(row, col) {
                Ok(_) => {
                    self.message = format!("在({},{})吃子成功", row, col);
                }
                Err(e) => {
                    self.message = format!("吃子失败: {}", e);
                }
            }
        }
        GamePhase::Movement => {
            if self.input_mode == InputMode::MovementFrom {
                // 选择要移动的棋子
                if let Cell::Occupied(p) = self.board.grid[row][col] {
                    if p == player {
                        self.selected_cell = Some((row, col));
                        self.input_mode = InputMode::MovementTo;
                        self.message = format!("已选择棋子({},{})，请选择目标位置", row, col);
                    } else {
                        self.message = "只能选择自己的棋子".to_string();
                    }
                } else {
                    self.message = "请选择有棋子的位置".to_string();
                }
            } else if self.input_mode == InputMode::MovementTo {
                // 选择目标位置
                if let Some(from) = self.selected_cell {
                    if from == (row, col) {
                        self.message = "不能移动到同一位置".to_string();
                        return;
                    }
                    
                    match self.board.move_piece(from, (row, col)) {
                        Ok(captured) => {
                            if captured > 0 {
                                self.message = format!("从({},{})移动到({},{})成功! 吃掉对方 {} 个棋子", 
                                    from.0, from.1, row, col, captured);
                            } else {
                                self.message = format!("从({},{})移动到({},{})成功!", 
                                    from.0, from.1, row, col);
                            }
                            self.selected_cell = None;
                            self.input_mode = InputMode::MovementFrom;
                        }
                        Err(e) => {
                            self.message = format!("移动失败: {}", e);
                            self.selected_cell = None;
                            self.input_mode = InputMode::MovementFrom;
                        }
                    }
                }
            }
        }
    }
    
    // 检查游戏是否结束
    if let Some(winner) = self.board.check_winner() {
        self.message = format!("游戏结束! {} 获胜!", winner);
        self.game_over = true;
    }
    
    // 更新输入模式
    let (new_phase, _) = self.board.get_state();
    if new_phase != phase {
        match new_phase {
            GamePhase::Placement => self.input_mode = InputMode::Placement,
            GamePhase::Capture => self.input_mode = InputMode::Capture,
            GamePhase::Movement => {
                self.input_mode = InputMode::MovementFrom;
                self.selected_cell = None;
            },
        }
        self.message = format!("进入{}", new_phase);
    }
}
    
    // 修改 draw_board 方法，添加 time 参数
    fn draw_board(&mut self, ui: &mut egui::Ui, time: f32) {
        let cell_size = 50.0;
        let padding = 30.0;
        let board_size = cell_size * 4.0 + padding * 2.0;
        
        // 创建棋盘画布
        let (response, painter) = ui.allocate_painter(
            egui::vec2(board_size, board_size),
            egui::Sense::click()
        );
        
        let rect = response.rect;
        
        // 绘制木质棋盘背景
        painter.rect_filled(rect, 5.0, Color32::from_rgb(188, 143, 101));
        
        // 绘制棋盘网格线
        for i in 0..5 {
            let x = rect.left() + padding + i as f32 * cell_size;
            painter.line_segment(
                [egui::pos2(x, rect.top() + padding), egui::pos2(x, rect.bottom() - padding)],
                Stroke::new(2.0, Color32::from_rgb(80, 50, 20))
            );
            
            let y = rect.top() + padding + i as f32 * cell_size;
            painter.line_segment(
                [egui::pos2(rect.left() + padding, y), egui::pos2(rect.right() - padding, y)],
                Stroke::new(2.0, Color32::from_rgb(80, 50, 20))
            );
        }
        
        // 绘制坐标
        for i in 0..5 {
            let x = rect.left() + padding + i as f32 * cell_size;
            painter.text(
                egui::pos2(x, rect.top() + padding - 20.0),
                Align2::CENTER_CENTER,
                &i.to_string(),
                FontId::proportional(16.0),
                Color32::from_rgb(50, 30, 10)
            );
            
            let y = rect.top() + padding + i as f32 * cell_size;
            painter.text(
                egui::pos2(rect.left() + padding - 20.0, y),
                Align2::CENTER_CENTER,
                &i.to_string(),
                FontId::proportional(16.0),
                Color32::from_rgb(50, 30, 10)
            );
        }
        
        // 绘制棋子
        for row in 0..5 {
            for col in 0..5 {
                let x = rect.left() + padding + col as f32 * cell_size;
                let y = rect.top() + padding + row as f32 * cell_size;
                let center = egui::pos2(x, y);
                
                match self.board.grid[row][col] {
                    Cell::Occupied(Player::Black) => {
                        // 绘制黑色棋子（带有光泽效果）
                        painter.circle_filled(center, cell_size / 3.0, Color32::from_rgb(40, 40, 40));
                        painter.circle_filled(center, cell_size / 3.5, Color32::from_rgb(20, 20, 20));
                        
                        // 添加高光
                        let highlight_pos = egui::pos2(x - cell_size/8.0, y - cell_size/8.0);
                        painter.circle_filled(highlight_pos, cell_size / 10.0, Color32::from_rgba_premultiplied(255, 255, 255, 100));
                    }
                    Cell::Occupied(Player::White) => {
                        // 绘制白色棋子（带有阴影效果）
                        painter.circle_filled(center, cell_size / 3.0, Color32::from_rgb(230, 230, 230));
                        painter.circle_stroke(center, cell_size / 3.0, Stroke::new(1.5, Color32::from_rgb(100, 100, 100)));
                        
                        // 添加阴影
                        let shadow_pos = egui::pos2(x + cell_size/10.0, y + cell_size/10.0);
                        painter.circle_filled(shadow_pos, cell_size / 3.1, Color32::from_rgba_premultiplied(0, 0, 0, 40));
                    }
                    Cell::Empty => {
                        // 在空位添加浅色圆点提示
                        painter.circle_filled(center, 3.0, Color32::from_rgba_premultiplied(0, 0, 0, 50));
                    }
                }
                
                // 高亮显示受保护的棋子
                let is_protected = if let Cell::Occupied(player) = self.board.grid[row][col] {
                    self.board.reward_pieces
                        .get(&player)
                        .map_or(false, |protected| protected.contains(&(row, col)))
                } else {
                    false
                };
                
                if is_protected {
                    painter.circle_stroke(center, cell_size / 2.8, Stroke::new(2.5, Color32::GOLD));
                    
                    // 添加保护标记（皇冠图标）
                    let crown_points = [
                        egui::pos2(x - 5.0, y - 3.0),
                        egui::pos2(x - 3.0, y - 7.0),
                        egui::pos2(x, y - 5.0),
                        egui::pos2(x + 3.0, y - 7.0),
                        egui::pos2(x + 5.0, y - 3.0),
                    ];
                    painter.add(eframe::egui::Shape::line(
                        crown_points.to_vec(),
                        Stroke::new(1.5, Color32::GOLD)
                    ));
                }
                
                // 高亮显示选中的棋子
                if self.selected_cell == Some((row, col)) {
                    painter.circle_stroke(center, cell_size / 2.8, Stroke::new(3.0, Color32::from_rgb(0, 150, 255)));
                    
                    // 添加脉动动画效果
                    let pulse = (time * 5.0).sin() * 2.0 + 2.0;
                    painter.circle_stroke(center, cell_size / 2.8 + pulse, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 150, 255, 100)));
                }
                
                // 高亮显示可移动的位置（在移动阶段）
                if self.input_mode == InputMode::MovementTo {
                    if let Some((from_row, from_col)) = self.selected_cell {
                        let row_diff = from_row.abs_diff(row);
                        let col_diff = from_col.abs_diff(col);
                        let is_adjacent = (row_diff == 1 && col_diff == 0) || (row_diff == 0 && col_diff == 1);
                        
                        if is_adjacent && self.board.grid[row][col] == Cell::Empty {
                            painter.circle_filled(center, 8.0, Color32::from_rgba_premultiplied(0, 255, 0, 100));
                        }
                    }
                }
            }
        }
        
        // 处理点击事件
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let col = ((pos.x - rect.left() - padding + cell_size / 2.0) / cell_size) as usize;
                let row = ((pos.y - rect.top() - padding + cell_size / 2.0) / cell_size) as usize;
                
                if row < 5 && col < 5 {
                    self.handle_cell_click(row, col);
                }
            }
        }
    }
}

// 更新 WudaoApp 的 update 方法
impl eframe::App for WudaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 更新时间用于动画
        self.time += ctx.input(|i| i.unstable_dt);
        
        // 设置窗口背景色
        ctx.set_visuals(eframe::egui::Visuals {
            window_fill: Color32::from_rgb(245, 235, 220),
            panel_fill: Color32::from_rgb(245, 235, 220),
            ..Default::default()
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(RichText::new("五道方游戏").color(Color32::from_rgb(120, 70, 30)).font(FontId::proportional(28.0)));
            
            // 游戏状态显示区域
            ui.add_space(10.0);
            let (phase, player) = self.board.get_state();
            
            // 创建状态面板
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(250, 245, 235))
                .stroke(Stroke::new(1.0, Color32::from_rgb(180, 150, 120)))
                .rounding(5.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("当前阶段:").font(FontId::proportional(16.0)).color(Color32::from_rgb(100, 60, 20)));
                        ui.label(RichText::new(format!("{}", phase)).font(FontId::proportional(16.0)).color(Color32::from_rgb(80, 40, 10)));
                        
                        ui.add_space(20.0);
                        
                        ui.label(RichText::new("当前玩家:").font(FontId::proportional(16.0)).color(Color32::from_rgb(100, 60, 20)));
                        ui.label(RichText::new(format!("{}", player)).font(FontId::proportional(16.0)).color(match player {
                            Player::Black => Color32::BLACK,
                            Player::White => Color32::from_rgb(80, 80, 80),
                        }));
                    });
                    
                    // 显示额外的游戏状态信息
                    match phase {
                        GamePhase::Placement if self.board.extra_moves > 0 => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("额外落子次数:").font(FontId::proportional(14.0)));
                                ui.label(RichText::new(format!("{}", self.board.extra_moves)).font(FontId::proportional(14.0)).color(Color32::DARK_GREEN));
                            });
                        }
                        GamePhase::Capture => {
                            let remaining = self.board.capture_remaining
                                .get(&player)
                                .copied()
                                .unwrap_or(0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("剩余吃子数量:").font(FontId::proportional(14.0)));
                                ui.label(RichText::new(format!("{}", remaining)).font(FontId::proportional(14.0)).color(Color32::DARK_RED));
                            });
                        }
                        GamePhase::Movement => {
                            if self.input_mode == InputMode::MovementFrom {
                                ui.label(RichText::new("请选择要移动的棋子").font(FontId::proportional(14.0)).color(Color32::DARK_BLUE));
                            } else if self.input_mode == InputMode::MovementTo {
                                ui.label(RichText::new("请选择目标位置").font(FontId::proportional(14.0)).color(Color32::DARK_BLUE));
                            }
                        }
                        _ => {}
                    }
                });
            
            ui.add_space(10.0);
            
            // 操作按钮区域
            ui.horizontal(|ui| {
                if ui.button(RichText::new("游戏规则").font(FontId::proportional(14.0))).clicked() {
                    self.show_help = !self.show_help;
                }
                
                if ui.button(RichText::new("认输").font(FontId::proportional(14.0))).clicked() {
                    self.message = format!("{} 认输，游戏结束！", player);
                    self.game_over = true;
                }
                
                if ui.button(RichText::new("新游戏").font(FontId::proportional(14.0))).clicked() {
                    *self = Self::new();
                }
                
                // 添加撤销按钮（如果支持的话）
                if ui.button(RichText::new("悔棋").font(FontId::proportional(14.0))).clicked() {
                    self.message = "悔棋功能尚未实现".to_string();
                }
            });
            
            ui.add_space(10.0);
            
            // 帮助提示
            if self.show_help {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgba_premultiplied(255, 255, 240, 200))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(220, 200, 100)))
                    .rounding(5.0)
                    .show(ui, |ui| {
                        ui.collapsing(RichText::new("游戏规则").font(FontId::proportional(16.0)).color(Color32::from_rgb(120, 70, 30)), |ui| {
                            ui.label(RichText::new("• 游戏分为三个阶段: 落子阶段、吃棋阶段、走子阶段").font(FontId::proportional(14.0)));
                            ui.label(RichText::new("• 落子阶段: 玩家轮流在5x5棋盘上放置棋子").font(FontId::proportional(14.0)));
                            ui.label(RichText::new("• 形成特定模式可获得奖励: 成方(+1子)、成三斜(+1子)、成四斜(+1子)、成州(+2子)、成龙(+2子)").font(FontId::proportional(14.0)));
                            ui.label(RichText::new("• 棋盘满后进入吃棋阶段: 后落子的玩家先吃棋，轮流吃掉对方棋子").font(FontId::proportional(14.0)));
                            ui.label(RichText::new("• 吃棋完成后进入走子阶段: 玩家轮流移动自己的棋子").font(FontId::proportional(14.0)));
                            ui.label(RichText::new("• 胜利条件: 对方棋子少于3个或无法移动时获胜").font(FontId::proportional(14.0)));
                        });
                    });
            }
            
            ui.add_space(10.0);
            
            // 显示消息
            if !self.message.is_empty() {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgba_premultiplied(240, 248, 255, 200))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(150, 180, 220)))
                    .rounding(5.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(&self.message).font(FontId::proportional(14.0)).color(Color32::from_rgb(50, 80, 120)));
                    });
            }
            
            ui.add_space(10.0);
            
            // 检查游戏是否结束
            if self.game_over {
                ui.heading(RichText::new("游戏结束!").color(Color32::from_rgb(180, 40, 40)).font(FontId::proportional(24.0)));
                return;
            }
            
            // 显示棋盘
            ui.vertical_centered(|ui| {
                self.draw_board(ui, self.time);
            });
            
            // 添加玩家提示
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(Color32::BLACK).font(FontId::proportional(20.0)));
                ui.label(RichText::new("黑方").font(FontId::proportional(14.0)));
                
                ui.add_space(20.0);
                
                ui.label(RichText::new("○").color(Color32::from_rgb(80, 80, 80)).font(FontId::proportional(20.0)));
                ui.label(RichText::new("白方").font(FontId::proportional(14.0)));
                
                ui.add_space(20.0);
                
                ui.label(RichText::new("⛁").color(Color32::GOLD).font(FontId::proportional(20.0)));
                ui.label(RichText::new("受保护棋子").font(FontId::proportional(14.0)));
            });
        });
        
        // 请求持续重绘以实现动画效果
        ctx.request_repaint();
    }
}
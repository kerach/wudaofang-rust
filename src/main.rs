use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use serde::{Serialize, Deserialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rand::seq::SliceRandom;
use rand::thread_rng;

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
    Tri { id: usize },                  // 成三斜
    Tetra { id: usize },                // 成四斜
    Row { index: usize },               // 成州(行)
    Col { index: usize },               // 成州(列)
    Dragon { id: usize },               // 成龙
}

impl fmt::Display for RewardPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewardPattern::Square { top_left: (r, c) } => 
                write!(f, "成方[位置:({},{})]", r, c),
            RewardPattern::Tri { id } => 
                write!(f, "成三斜[模式:{}]", id),
            RewardPattern::Tetra { id } => 
                write!(f, "成四斜[模式:{}]", id),
            RewardPattern::Row { index } => 
                write!(f, "成州[行:{}]", index),
            RewardPattern::Col { index } => 
                write!(f, "成州[列:{}]", index),
            RewardPattern::Dragon { id } => 
                write!(f, "成龙[对角线:{}]", id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameAction {
    Place { player: Player, pos: (usize, usize) },
    Capture { player: Player, pos: (usize, usize) },
    Move { player: Player, from: (usize, usize), to: (usize, usize) },
    Reward { player: Player, pattern: RewardPattern },
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
    capture_turn: Player, // 当前吃棋玩家
    // 用于记录已触发的奖励模式
    triggered_squares: HashSet<[usize; 2]>, // 成方 [左上角行, 左上角列]
    triggered_tris: HashSet<usize>,        // 成三斜 0-3
    triggered_tetras: HashSet<usize>,      // 成四斜 0-3
    triggered_rows: HashSet<usize>,        // 成州行 0-4
    triggered_cols: HashSet<usize>,        // 成州列 0-4
    triggered_dragons: HashSet<usize>,     // 成龙 0-1
    // 奖励模式保护的棋子
    reward_pieces: HashMap<Player, HashSet<(usize, usize)>>,
    // 游戏记录
    game_record: Vec<GameAction>,
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
        };
        
        // 初始化奖励棋子保护集
        board.update_reward_pieces();
        board
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
                let remaining = self.capture_remaining.get(&self.current_player)
                    .copied().unwrap_or(0);
                println!("剩余吃子数量: {}", remaining);
                println!("提示: 请输入吃子位置 (格式: 行,列), 例如: 1,2");
                println!("注意: 不能吃受保护棋子（在奖励模式中的棋子）");
            }
            GamePhase::Movement => {
                println!("提示: 请输入要移动的棋子位置和目标位置 (格式: 原行,原列 新行,新列), 例如: 1,2 1,3");
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
        self.grid.iter().all(|row| row.iter().all(|c| *c != Cell::Empty))
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
            let neighbors = [(r.wrapping_sub(1), c), (r + 1, c), (r, c.wrapping_sub(1)), (r, c + 1)];
            
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
            pos: (row, col)
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
        
        // 重置奖励模式记录
        self.triggered_squares.clear();
        self.triggered_tris.clear();
        self.triggered_tetras.clear();
        self.triggered_rows.clear();
        self.triggered_cols.clear();
        self.triggered_dragons.clear();
        
        // 重新计算所有奖励模式
        self.scan_all_rewards();
        
        // 设置吃棋顺序：后落子的玩家先吃棋
        let last_player = self.current_player.opponent();
        let first_player = last_player; // 后落子的玩家先吃棋
        let second_player = last_player.opponent(); // 先落子的玩家后吃棋
        
        // 计算吃子数量
        let first_capture = self.calculate_capture_count(first_player);
        let second_capture = self.calculate_capture_count(second_player);
        
        // 设置吃棋队列
        self.capture_remaining.insert(first_player, first_capture);
        self.capture_remaining.insert(second_player, second_capture);
        self.capture_turn = first_player;
        self.current_player = first_player;
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
        count += self.triggered_rows.iter()
            .filter(|&&r| self.is_row(r, player))
            .count() as u32 * 2;
        
        count += self.triggered_cols.iter()
            .filter(|&&c| self.is_col(c, player))
            .count() as u32 * 2;
        
        // 成龙数量
        count += self.triggered_dragons.iter()
            .filter(|&&id| self.is_dragon(id, player))
            .count() as u32 * 2;
        
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
        let protected = self.reward_pieces.get(&opponent).cloned().unwrap_or_default();
        
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
            pos: (row, col)
        });
        
        // 更新吃棋剩余数量
        *self.capture_remaining.get_mut(&player).unwrap() = remaining - 1;
        
        // 更新奖励棋子保护集
        self.update_reward_pieces();
        
        // 检查吃棋后状态
        if self.capture_remaining.values().sum::<u32>() == 0 {
            // 所有吃棋完成，进入移动阶段
            self.enter_movement_phase();
            return Ok(());
        }
        
        // 切换到下一个吃棋玩家
        let next_player = player.opponent();
        
        // 如果下一个玩家没有可吃的棋子，则跳过
        if self.player_pieces(opponent).is_empty() {
            *self.capture_remaining.get_mut(&next_player).unwrap() = 0;
            
            // 检查是否所有玩家都完成吃棋
            if self.capture_remaining.values().sum::<u32>() == 0 {
                self.enter_movement_phase();
                 return Ok(());
            } else {
                self.current_player = next_player.opponent();
                return Ok(());
            }
        } else {
            self.current_player = next_player;
        }
        
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
    fn enter_movement_phase(&mut self) {
        self.phase = GamePhase::Movement;
        // 移动阶段的先手是吃棋阶段的后吃棋玩家
           if let Some((last_capture_player, _)) = self.capture_remaining.iter().last() {
            self.current_player = *last_capture_player;
        }
        self.update_reward_pieces();
    }

    // 执行移动
    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> Result<u32, &'static str> {
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
        let capture_count = self.check_rewards_movement(to_row, to_col);
        
        // 吃子操作
        let mut actual_captured = 0;
        if capture_count > 0 {
            let opponent = player.opponent();
            let opponent_pieces = self.player_pieces(opponent);
            let protected = self.reward_pieces.get(&opponent).cloned().unwrap_or_default();
            
            // 找出可吃的棋子（不在保护集中的）
            let mut capturable: Vec<_> = opponent_pieces
                .into_iter()
                .filter(|pos| !protected.contains(pos))
                .collect();
            
            // 吃子数量不超过可吃棋子数量
            actual_captured = capture_count.min(capturable.len() as u32);
            
            // 执行吃子（移除棋子）
            for _ in 0..actual_captured {
                if let Some(pos) = capturable.pop() {
                    self.grid[pos.0][pos.1] = Cell::Empty;
                    
                    // 记录吃棋动作
                    self.record_action(GameAction::Capture {
                        player,
                        pos: (pos.0, pos.1)
                    });
                }
            }
            
            // 更新奖励棋子保护集
            self.update_reward_pieces();
        }
        
        // 检查移动后对方是否能走棋
        let opponent = player.opponent();
        if !self.has_legal_moves(opponent) {
            // 导致对方无法走棋，当前玩家判负
            return Err("移动导致对方无法走棋，你输了");
        }
        
        // 切换玩家
        self.current_player = self.current_player.opponent();
        
        Ok(actual_captured)
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
                        pattern: RewardPattern::Square {
                            top_left: (r, c)
                        }
                    });
                    println!("玩家 {} 形成成方[位置:({},{})]", player, r, c);
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
                    pattern: RewardPattern::Tri { id }
                });
                println!("玩家 {} 形成成三斜[模式:{}]", player, id);
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
                    pattern: RewardPattern::Tetra { id }
                });
                println!("玩家 {} 形成成四斜[模式:{}]", player, id);
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
                    pattern: RewardPattern::Row { index: r }
                });
                println!("玩家 {} 形成成州[行:{}]", player, r);
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
                    pattern: RewardPattern::Col { index: c }
                });
                println!("玩家 {} 形成成州[列:{}]", player, c);
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
                    pattern: RewardPattern::Dragon { id }
                });
                println!("玩家 {} 形成成龙[对角线:{}]", player, id);
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
            (row, col), (row, col.saturating_sub(1)), 
            (row.saturating_sub(1), col), (row.saturating_sub(1), col.saturating_sub(1))
        ] {
            if r < 4 && c < 4 {
                if self.is_square(r, c, player) {
                    let square_id = [r, c];
                    if self.triggered_squares.insert(square_id) {
                        extra += 1;
                        
                        // 记录奖励模式
                        self.record_action(GameAction::Reward {
                            player,
                            pattern: RewardPattern::Square {
                                top_left: (r, c)
                            }
                        });
                    }
                }
            }
        }
        extra
    }
    
    fn is_square(&self, r: usize, c: usize, player: Player) -> bool {
        let corners = [
            (r, c), (r, c + 1), 
            (r + 1, c), (r + 1, c + 1)
        ];
        
        corners.iter().all(|&(r, c)| {
            matches!(self.grid[r][c], Cell::Occupied(p) if p == player)
        })
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
                    pattern: RewardPattern::Tri { id }
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
        
        positions.iter().all(|&(r, c)| {
            matches!(self.grid[r][c], Cell::Occupied(p) if p == player)
        })
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
                    pattern: RewardPattern::Tetra { id }
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
        
        positions.iter().all(|&(r, c)| {
            matches!(self.grid[r][c], Cell::Occupied(p) if p == player)
        })
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
                    pattern: RewardPattern::Row { index: r }
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
                    pattern: RewardPattern::Col { index: c }
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
                    pattern: RewardPattern::Dragon { id }
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
        
        positions.iter().all(|&(r, c)| {
            matches!(self.grid[r][c], Cell::Occupied(p) if p == player)
        })
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
    
    let row = parts[0].parse::<usize>()
        .map_err(|_| "行号必须是0-4之间的数字")?;
    let col = parts[1].parse::<usize>()
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
fn main() {
    println!("\n===== 欢迎来到五道方游戏! =====");
    println!("游戏规则说明:");
    println!("1. 游戏分为三个阶段: 落子阶段、吃棋阶段、走子阶段");
    println!("2. 落子阶段: 玩家轮流在5x5棋盘上放置棋子");
    println!("3. 形成特定模式可获得奖励: 成方(+1子)、成三斜(+1子)、成四斜(+1子)、成州(+2子)、成龙(+2子)");
    println!("4. 棋盘满后进入吃棋阶段: 后落子的玩家先吃棋，轮流吃掉对方棋子");
    println!("5. 吃棋完成后进入走子阶段: 玩家轮流移动自己的棋子");
    println!("6. 胜利条件: 对方棋子少于3个或无法移动时获胜");
    println!("================================\n");
    
    let mut board = Board::new();
    let mut game_over = false;
    
    while !game_over {
        board.print_board();
        board.print_game_status();

           // 添加AI推荐（无侵入性）
        let recommendation = BoardAI::recommend_move(&board);
        println!("AI推荐: {}", recommendation);
        
        // 检查胜利条件（只在吃棋和走子阶段）
        if let Some(winner) = board.check_winner() {
            println!("\n===== 游戏结束! =====");
            println!("{} 获胜!", winner);
            game_over = true;
            continue;
        }
        
        match board.phase {
            GamePhase::Placement => {
                let input = read_input(&format!("{} 请输入落子位置: ", board.current_player));
                
                match parse_coord(&input) {
                    Ok((row, col)) => {
                        match board.place_piece(row, col) {
                            Ok(extra) => {
                                if extra > 0 {
                                    println!("{} 形成奖励模式，获得额外落子次数: {}", board.current_player, extra);
                                }
                            }
                            Err(e) => println!("操作失败: {}", e),
                        }
                    }
                    Err(e) => println!("输入错误: {}", e),
                }
            }
            
      GamePhase::Capture => {
    // 检查当前玩家是否可以吃子
    let opponent = board.current_player.opponent();
    let opponent_pieces = board.player_pieces(opponent);
    let protected = board.reward_pieces.get(&opponent).cloned().unwrap_or_default();
    
    // 找出可吃的棋子（不在保护集中的）
    let capturable: Vec<_> = opponent_pieces
        .iter()
        .filter(|pos| !protected.contains(pos))
        .collect();
    
    // 如果当前玩家有吃子任务但无棋子可吃，自动跳过
    if capturable.is_empty() {
        let player = board.current_player;
        let remaining = board.capture_remaining.get(&player).copied().unwrap_or(0);
        
        if remaining > 0 {
            println!("{} 没有可吃的棋子，自动跳过吃棋阶段", player);
            
            // 清零当前玩家的吃子任务
            *board.capture_remaining.get_mut(&player).unwrap() = 0;
            
            // 检查是否所有玩家都完成吃棋
            if board.capture_remaining.values().sum::<u32>() == 0 {
                board.enter_movement_phase();
                continue;
            }
            
            // 切换到下一个玩家
            board.current_player = player.opponent();
            continue;
        }
    }
    
    // 正常处理吃棋输入
    let input = read_input(&format!("{} 请输入吃子位置: ", board.current_player));
    
    match parse_coord(&input) {
        Ok((row, col)) => {
            match board.capture_piece(row, col) {
                Ok(_) => println!("吃棋成功!"),
                Err(e) => println!("操作失败: {}", e),
            }
        }
        Err(e) => println!("输入错误: {}", e),
    }
}
            
            GamePhase::Movement => {
                let input = read_input(&format!("{} 请输入移动指令 (原位置 目标位置): ", board.current_player));
                
                match parse_move(&input) {
                    Ok((from, to)) => {
                        match board.move_piece(from, to) {
                            Ok(captured) => {
                                if captured > 0 {
                                    println!("移动成功! 吃掉对方 {} 个棋子", captured);
                                } else {
                                    println!("移动成功!");
                                }
                            }
                            Err(e) => println!("操作失败: {}", e),
                        }
                    }
                    Err(e) => println!("输入错误: {}", e),
                }
            }
        }
    }
    
    // 保存棋谱选项
    let save = read_input("是否保存棋谱? (y/n): ");
    if save.to_lowercase() == "y" {
        let serialized = serde_json::to_string_pretty(board.get_game_record()).unwrap();
        let filename = "wudao_game_record.json";
        std::fs::write(filename, &serialized).unwrap();
        println!("棋谱已保存到 {}", filename);
    }
    
    println!("\n感谢游玩五道方游戏！");
}


// 无侵入性的AI推荐器
pub struct BoardAI;

impl BoardAI {
    // 主推荐函数
    pub fn recommend_move(board: &Board) -> String {
        // 根据游戏阶段选择不同的推荐策略
        match board.phase {
            GamePhase::Placement => Self::recommend_placement(board),
            GamePhase::Capture => Self::recommend_capture(board),
            GamePhase::Movement => Self::recommend_movement(board),
        }
    }

    // 落子阶段推荐
    fn recommend_placement(board: &Board) -> String {
        let mut best_score = i32::MIN;
        let mut best_pos = (0, 0);
        
        for r in 0..5 {
            for c in 0..5 {
                if board.grid[r][c] == Cell::Empty {
                    // 评估这个落子位置
                    let score = Self::evaluate_placement(board, (r, c));
                    
                    if score > best_score {
                        best_score = score;
                        best_pos = (r, c);
                    }
                }
            }
        }
        
        format!("推荐落子位置: {},{}", best_pos.0, best_pos.1)
    }

    // 评估落子位置
    fn evaluate_placement(board: &Board, pos: (usize, usize)) -> i32 {
        let player = board.current_player;
        let mut score = 0;
        
        // 1. 基础价值：落子本身的价值
        score += 10;
        
        // 2. 奖励模式潜力
        score += Self::reward_potential(board, pos, player) * 50;
        
        // 3. 位置价值：中心位置更有价值
        score += Self::position_value(board, pos, player);
        
        // 4. 威胁评估：防止对手形成奖励
        score -= Self::threat_assessment(board, pos, player) * 20;
        
        score
    }

    // 吃棋阶段推荐
    fn recommend_capture(board: &Board) -> String {
        let player = board.current_player;
        let opponent = player.opponent();
        let protected = board.reward_pieces.get(&opponent).cloned().unwrap_or_default();
        let mut best_score = i32::MIN;
        let mut best_pos = (0, 0);
        
        for r in 0..5 {
            for c in 0..5 {
                if let Cell::Occupied(p) = board.grid[r][c] {
                    if p == opponent && !protected.contains(&(r, c)) {
                        // 评估吃这个棋子的价值
                        let score = Self::evaluate_capture(board, (r, c));
                        
                        if score > best_score {
                            best_score = score;
                            best_pos = (r, c);
                        }
                    }
                }
            }
        }
        
        format!("推荐吃子位置: {},{}", best_pos.0, best_pos.1)
    }

    // 评估吃子位置
    fn evaluate_capture(board: &Board, pos: (usize, usize)) -> i32 {
        let mut score = 10;  // 基本吃子价值
        
        // 中心位置价值更高
        if pos.0 == 2 && pos.1 == 2 {
            score += 5;
        }
        
        // 破坏对手潜在模式
        if Self::is_potential_reward_piece(board, pos) {
            score += 15;
        }
        
        score
    }

    // 走棋阶段推荐
    fn recommend_movement(board: &Board) -> String {
        let player = board.current_player;
        let mut best_score = i32::MIN;
        let mut best_move = ((0, 0), (0, 0));
        
        // 使用蒙特卡洛树搜索
        let time_limit = Duration::from_secs(1);
        if let Some((from, to)) = Self::monte_carlo_search(board, time_limit) {
            best_move = (from, to);
        }
        
        format!(
            "推荐移动: {},{} 移动到 {},{}", 
            best_move.0.0, best_move.0.1, 
            best_move.1.0, best_move.1.1
        )
    }

    // 蒙特卡洛树搜索
    fn monte_carlo_search(board: &Board, time_limit: Duration) -> Option<((usize, usize), (usize, usize))> {
        let start_time = Instant::now();
        let mut best_move = None;
        let mut best_score = i32::MIN;
        
        // 获取所有可能的移动
        let moves = Self::generate_possible_moves(board);
        
        // 并行评估移动
        let moves_ref = Arc::new(Mutex::new((best_move, best_score)));
        let board_clone = board.clone();
        
        thread::scope(|s| {
            for mv in moves {
                let moves_ref = Arc::clone(&moves_ref);
                let board_clone = board_clone.clone();
                
                s.spawn(move || {
                    let mut total_score = 0;
                    let mut simulations = 0;
                    
                    // 在时间限制内进行模拟
                    while start_time.elapsed() < time_limit {
                        let mut sim_board = board_clone.clone();
                        if sim_board.move_piece(mv.0, mv.1).is_ok() {
                            // 模拟随机游戏
                            let score = Self::simulate_random_game(&sim_board);
                            total_score += score;
                            simulations += 1;
                        }
                    }
                    
                    if simulations > 0 {
                        let avg_score = total_score / simulations;
                        
                        let mut lock = moves_ref.lock().unwrap();
                        if avg_score > lock.1 {
                            lock.1 = avg_score;
                            lock.0 = Some(mv);
                        }
                    }
                });
            }
        });
        
        let lock = moves_ref.lock().unwrap();
        lock.0
    }

    // 生成所有可能的移动
    fn generate_possible_moves(board: &Board) -> Vec<((usize, usize), (usize, usize))> {
        let player = board.current_player;
        let mut moves = Vec::new();
        
        for (r, c) in board.player_pieces(player) {
            // 检查四个方向
            let neighbors = [
                (r.wrapping_sub(1), c),
                (r + 1, c),
                (r, c.wrapping_sub(1)),
                (r, c + 1),
            ];
            
            for (nr, nc) in neighbors {
                if Board::is_valid_pos(nr, nc) && board.grid[nr][nc] == Cell::Empty {
                    moves.push(((r, c), (nr, nc)));
                }
            }
        }
        
        moves
    }

    // 模拟随机游戏
    fn simulate_random_game(board: &Board) -> i32 {
        let mut sim_board = board.clone();
        let original_player = sim_board.current_player;
        let mut depth = 0;
        let max_depth = 20;
        
        while depth < max_depth {
            if let Some(winner) = sim_board.check_winner() {
                return if winner == original_player { 100 } else { -100 };
            }
            
            // 根据游戏阶段选择随机动作
            match sim_board.phase {
                GamePhase::Placement => {
                    let empty_cells: Vec<_> = (0..5)
                        .flat_map(|r| (0..5).map(move |c| (r, c)))
                        .filter(|&(r, c)| sim_board.grid[r][c] == Cell::Empty)
                        .collect();
                    
                    if let Some(&(r, c)) = empty_cells.choose(&mut thread_rng()) {
                        let _ = sim_board.place_piece(r, c);
                    }
                }
                GamePhase::Capture => {
                    let opponent = sim_board.current_player.opponent();
                    let protected = sim_board.reward_pieces
                        .get(&opponent)
                        .cloned()
                        .unwrap_or_default();
                    
                    let capturable: Vec<_> = sim_board.player_pieces(opponent)
                        .into_iter()
                        .filter(|pos| !protected.contains(pos))
                        .collect();
                    
                    if let Some(&(r, c)) = capturable.choose(&mut thread_rng()) {
                        let _ = sim_board.capture_piece(r, c);
                    }
                }
                GamePhase::Movement => {
                    let moves = Self::generate_possible_moves(&sim_board);
                    if let Some(&(from, to)) = moves.choose(&mut thread_rng()) {
                        let _ = sim_board.move_piece(from, to);
                    }
                }
            }
            
            depth += 1;
        }
        
        // 评估最终局势
        Self::evaluate_position(&sim_board, original_player)
    }

    // 评估局势
    fn evaluate_position(board: &Board, player: Player) -> i32 {
        let opponent = player.opponent();
        
        // 1. 棋子数量差异
        let player_pieces = board.player_pieces(player).len() as i32;
        let opponent_pieces = board.player_pieces(opponent).len() as i32;
        let mut score = (player_pieces - opponent_pieces) * 10;
        
        // 2. 奖励模式价值
        score += Self::count_rewards(board, player) * 50;
        score -= Self::count_rewards(board, opponent) * 50;
        
        // 3. 保护棋子价值
        score += Self::count_protected(board, player) * 5;
        score -= Self::count_protected(board, opponent) * 5;
        
        // 4. 位置控制价值
        score += Self::position_control_value(board, player);
        
        score
    }

    // 计算奖励模式数量
    fn count_rewards(board: &Board, player: Player) -> i32 {
        let mut count = 0;
        
        // 成方
        for r in 0..4 {
            for c in 0..4 {
                if board.is_square(r, c, player) {
                    count += 1;
                }
            }
        }
        
        // 成三斜
        for id in 0..4 {
            if board.is_tri(id, player) {
                count += 1;
            }
        }
        
        // 成四斜
        for id in 0..4 {
            if board.is_tetra(id, player) {
                count += 1;
            }
        }
        
        // 成州
        for r in 0..5 {
            if board.is_row(r, player) {
                count += 2;
            }
        }
        for c in 0..5 {
            if board.is_col(c, player) {
                count += 2;
            }
        }
        
        // 成龙
        for id in 0..2 {
            if board.is_dragon(id, player) {
                count += 2;
            }
        }
        
        count
    }

    // 计算受保护棋子数量
    fn count_protected(board: &Board, player: Player) -> i32 {
        board.reward_pieces
            .get(&player)
            .map_or(0, |set| set.len() as i32)
    }

    // 评估位置控制价值
    fn position_control_value(board: &Board, player: Player) -> i32 {
        let mut value = 0;
        let center_weights = [
            [1, 2, 3, 2, 1],
            [2, 4, 6, 4, 2],
            [3, 6, 9, 6, 3],
            [2, 4, 6, 4, 2],
            [1, 2, 3, 2, 1],
        ];
        
        for (r, row) in board.grid.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if let Cell::Occupied(p) = cell {
                    if *p == player {
                        value += center_weights[r][c];
                    }
                }
            }
        }
        
        value
    }




 // 改进的位置价值评估
    fn position_value(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let (r, c) = pos;
        let opponent = player.opponent();
        let mut value = 0;
        
        // 1. 基础位置权重（考虑棋盘对称性）
        let position_weights = [
            [1, 2, 3, 2, 1],
            [2, 4, 6, 4, 2],
            [3, 6, 9, 6, 3],
            [2, 4, 6, 4, 2],
            [1, 2, 3, 2, 1],
        ];
        value += position_weights[r][c];
        
        // 2. 奖励模式参与度评估
        value += Self::reward_participation_value(board, pos, player) * 5;
        value -= Self::reward_participation_value(board, pos, opponent) * 5;
        
        // 3. 战略位置价值（根据当前局势动态调整）
        if Self::is_strategic_pivot(board, pos) {
            value += 8;
        }
        
        // 4. 连接性价值（与其他棋子的连接程度）
        value += Self::connection_value(board, pos, player) * 3;
        
        value
    }

    // 评估位置在奖励模式中的参与度
    fn reward_participation_value(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let mut participation = 0;
        
        // 检查位置可能参与的所有奖励模式
        participation += Self::square_participation(board, pos, player);
        participation += Self::diagonal_participation(board, pos, player);
        participation += Self::row_col_participation(board, pos, player);
        
        participation
    }

    // 检查位置在成方模式中的参与度
    fn square_participation(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let (r, c) = pos;
        let mut participation = 0;
        
        // 检查可能包含该位置的所有1x1正方形
        for top_r in r.saturating_sub(1)..=r {
            for left_c in c.saturating_sub(1)..=c {
                if top_r < 4 && left_c < 4 {
                    let positions = [
                        (top_r, left_c),
                        (top_r, left_c + 1),
                        (top_r + 1, left_c),
                        (top_r + 1, left_c + 1),
                    ];
                    
                    // 计算玩家在这个正方形中的棋子数量
                    let player_count = positions.iter()
                        .filter(|&&(r, c)| {
                            matches!(board.grid[r][c], Cell::Occupied(p) if p == player)
                        })
                        .count() as i32;
                    
                    // 计算空位数量
                    let empty_count = positions.iter()
                        .filter(|&&(r, c)| board.grid[r][c] == Cell::Empty)
                        .count() as i32;
                    
                    // 如果位置在正方形中且模式未完成
                    if positions.contains(&pos) && player_count < 4 {
                        participation += player_count + empty_count;
                    }
                }
            }
        }
        
        participation
    }

    // 检查位置在斜线模式中的参与度
    fn diagonal_participation(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let mut participation = 0;
        
        // 所有可能的斜线模式
        let diagonals = [
            // 成三斜
            vec![(0, 2), (1, 1), (2, 0)],
            vec![(0, 2), (1, 3), (2, 4)],
            vec![(2, 0), (3, 1), (4, 2)],
            vec![(2, 4), (3, 3), (4, 2)],
            // 成四斜
            vec![(0, 1), (1, 2), (2, 3), (3, 4)],
            vec![(0, 3), (1, 2), (2, 1), (3, 0)],
            vec![(1, 0), (2, 1), (3, 2), (4, 3)],
            vec![(1, 4), (2, 3), (3, 2), (4, 1)],
            // 成龙
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)],
            vec![(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)],
        ];
        
        for pattern in diagonals {
            if pattern.contains(&pos) {
                let player_count = pattern.iter()
                    .filter(|&&(r, c)| {
                        matches!(board.grid[r][c], Cell::Occupied(p) if p == player)
                    })
                    .count() as i32;
                
                let empty_count = pattern.iter()
                    .filter(|&&(r, c)| board.grid[r][c] == Cell::Empty)
                    .count() as i32;
                
                participation += player_count + empty_count;
            }
        }
        
        participation
    }

    // 检查位置在行/列模式中的参与度
    fn row_col_participation(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let (r, c) = pos;
        let mut participation = 0;
        
        // 检查所在行
        let row_player_count = (0..5).filter(|&col| 
            matches!(board.grid[r][col], Cell::Occupied(p) if p == player)
        ).count() as i32;
        
        let row_empty_count = (0..5).filter(|&col| 
            board.grid[r][col] == Cell::Empty
        ).count() as i32;
        
        // 检查所在列
        let col_player_count = (0..5).filter(|&row| 
            matches!(board.grid[row][c], Cell::Occupied(p) if p == player)
        ).count() as i32;
        
        let col_empty_count = (0..5).filter(|&row| 
            board.grid[row][c] == Cell::Empty
        ).count() as i32;
        
        participation += row_player_count + row_empty_count;
        participation += col_player_count + col_empty_count;
        
        participation
    }

    // 判断是否是战略支点位置
    fn is_strategic_pivot(board: &Board, pos: (usize, usize)) -> bool {
        let (r, c) = pos;
        
        // 1. 中心位置总是重要的
        if r == 2 && c == 2 {
            return true;
        }
        
        // 2. 检查是否连接多个奖励模式
        let connected_patterns = Self::count_connected_patterns(board, pos);
        if connected_patterns >= 2 {
            return true;
        }
        
        // 3. 检查是否在敌我争夺区域
        Self::is_contested_area(board, pos)
    }

    // 计算位置连接的奖励模式数量
    fn count_connected_patterns(board: &Board, pos: (usize, usize)) -> i32 {
        let mut count = 0;
        
        // 检查所有可能包含此位置的模式
        count += Self::count_squares_containing(board, pos);
        count += Self::count_diagonals_containing(board, pos);
        
        // 行和列各算一种
        count += 2;
        
        count
    }

    // 计算包含位置的成方数量
    fn count_squares_containing(board: &Board, pos: (usize, usize)) -> i32 {
        let (r, c) = pos;
        let mut count = 0;
        
        for top_r in r.saturating_sub(1)..=r {
            for left_c in c.saturating_sub(1)..=c {
                if top_r < 4 && left_c < 4 {
                    let square = [
                        (top_r, left_c),
                        (top_r, left_c + 1),
                        (top_r + 1, left_c),
                        (top_r + 1, left_c + 1),
                    ];
                    
                    if square.contains(&pos) {
                        count += 1;
                    }
                }
            }
        }
        
        count
    }

    // 计算包含位置的斜线模式数量
    fn count_diagonals_containing(board: &Board, pos: (usize, usize)) -> i32 {
        let diagonals = [
            // 成三斜
            vec![(0, 2), (1, 1), (2, 0)],
            vec![(0, 2), (1, 3), (2, 4)],
            vec![(2, 0), (3, 1), (4, 2)],
            vec![(2, 4), (3, 3), (4, 2)],
            // 成四斜
            vec![(0, 1), (1, 2), (2, 3), (3, 4)],
            vec![(0, 3), (1, 2), (2, 1), (3, 0)],
            vec![(1, 0), (2, 1), (3, 2), (4, 3)],
            vec![(1, 4), (2, 3), (3, 2), (4, 1)],
            // 成龙
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)],
            vec![(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)],
        ];
        
        diagonals.iter()
            .filter(|pattern| pattern.contains(&pos))
            .count() as i32
    }

    // 判断是否是敌我争夺区域
    fn is_contested_area(board: &Board, pos: (usize, usize)) -> bool {
        let (r, c) = pos;
        let player = board.current_player;
        let opponent = player.opponent();
        
        // 检查2x2区域内的棋子分布
        let mut player_count = 0;
        let mut opponent_count = 0;
        
        for dr in 0..=1 {
            for dc in 0..=1 {
                let nr = r + dr;
                let nc = c + dc;
                
                if nr < 5 && nc < 5 {
                    match board.grid[nr][nc] {
                        Cell::Occupied(p) if p == player => player_count += 1,
                        Cell::Occupied(p) if p == opponent => opponent_count += 1,
                        _ => {}
                    }
                }
            }
        }
        
        // 如果双方都有棋子存在，则是争夺区域
        player_count > 0 && opponent_count > 0
    }

    // 评估位置与其他棋子的连接性
    fn connection_value(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let (r, c) = pos;
        let mut connections = 0;
        
        // 检查四个方向
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        
        for (dr, dc) in directions {
            let mut nr = r as i32 + dr;
            let mut nc = c as i32 + dc;
            let mut found_player = false;
            let mut found_opponent = false;
            
            // 沿方向寻找玩家棋子
            while nr >= 0 && nr < 5 && nc >= 0 && nc < 5 {
                match board.grid[nr as usize][nc as usize] {
                    Cell::Occupied(p) if p == player => {
                        found_player = true;
                        break;
                    }
                    Cell::Occupied(p) if p == player.opponent() => {
                        found_opponent = true;
                        break;
                    }
                    _ => {}
                }
                
                nr += dr;
                nc += dc;
            }
            
            if found_player {
                connections += 1;
            } else if !found_opponent {
                // 如果方向没有对手棋子，增加潜在连接价值
                connections += 1;
            }
        }
        
        connections
    }

    // 改进的威胁评估
    fn threat_assessment(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let opponent = player.opponent();
        let mut threats = 0;
        
        // 1. 直接威胁（对手可能立即形成的奖励）
        threats += Self::immediate_threats(board, pos, opponent) * 3;
        
        // 2. 潜在威胁（对手可能在未来形成的奖励）
        threats += Self::potential_threats(board, pos, opponent);
        
        // 3. 位置控制威胁（对手可能利用此位置）
        threats += Self::positional_threats(board, pos, opponent);
        
        threats
    }

    // 评估直接威胁
    fn immediate_threats(board: &Board, pos: (usize, usize), opponent: Player) -> i32 {
        let mut threats = 0;
        
        // 检查所有包含此位置的模式
        threats += Self::square_threats(board, pos, opponent);
        threats += Self::diagonal_threats(board, pos, opponent);
        threats += Self::row_col_threats(board, pos, opponent);
        
        threats
    }

    // 评估成方威胁
    fn square_threats(board: &Board, pos: (usize, usize), opponent: Player) -> i32 {
        let (r, c) = pos;
        let mut threats = 0;
        
        for top_r in r.saturating_sub(1)..=r {
            for left_c in c.saturating_sub(1)..=c {
                if top_r < 4 && left_c < 4 {
                    let square = [
                        (top_r, left_c),
                        (top_r, left_c + 1),
                        (top_r + 1, left_c),
                        (top_r + 1, left_c + 1),
                    ];
                    
                    if square.contains(&pos) {
                        let opponent_count = square.iter()
                            .filter(|&&(r, c)| {
                                matches!(board.grid[r][c], Cell::Occupied(p) if p == opponent)
                            })
                            .count() as i32;
                        
                        let empty_count = square.iter()
                            .filter(|&&(r, c)| board.grid[r][c] == Cell::Empty)
                            .count() as i32;
                        
                        // 如果对手差一个棋子就能形成成方
                        if opponent_count == 3 && empty_count == 1 {
                            threats += 1;
                        }
                    }
                }
            }
        }
        
        threats
    }

    // 评估斜线威胁
    fn diagonal_threats(board: &Board, pos: (usize, usize), opponent: Player) -> i32 {
        let diagonals = [
            // 成三斜
            vec![(0, 2), (1, 1), (2, 0)],
            vec![(0, 2), (1, 3), (2, 4)],
            vec![(2, 0), (3, 1), (4, 2)],
            vec![(2, 4), (3, 3), (4, 2)],
            // 成四斜
            vec![(0, 1), (1, 2), (2, 3), (3, 4)],
            vec![(0, 3), (1, 2), (2, 1), (3, 0)],
            vec![(1, 0), (2, 1), (3, 2), (4, 3)],
            vec![(1, 4), (2, 3), (3, 2), (4, 1)],
            // 成龙
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)],
            vec![(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)],
        ];
        
        let mut threats = 0;
        
        for pattern in diagonals {
            if pattern.contains(&pos) {
                let opponent_count = pattern.iter()
                    .filter(|&&(r, c)| {
                        matches!(board.grid[r][c], Cell::Occupied(p) if p == opponent)
                    })
                    .count() as i32;
                
                let empty_count = pattern.iter()
                    .filter(|&&(r, c)| board.grid[r][c] == Cell::Empty)
                    .count() as i32;
                
                // 威胁程度取决于模式大小和完成度
                let pattern_size = pattern.len() as i32;
                if opponent_count >= pattern_size - 1 && empty_count <= 1 {
                    threats += pattern_size; // 更大的模式威胁更大
                }
            }
        }
        
        threats
    }

    // 评估行列威胁
    fn row_col_threats(board: &Board, pos: (usize, usize), opponent: Player) -> i32 {
        let (r, c) = pos;
        let mut threats = 0;
        
        // 检查行威胁
        let row_opponent_count = (0..5).filter(|&col| 
            matches!(board.grid[r][col], Cell::Occupied(p) if p == opponent)
        ).count() as i32;
        
        // 检查列威胁
        let col_opponent_count = (0..5).filter(|&row| 
            matches!(board.grid[row][c], Cell::Occupied(p) if p == opponent)
        ).count() as i32;
        
        // 如果一行/列中对手有3个或更多棋子，存在威胁
        if row_opponent_count >= 3 {
            threats += row_opponent_count;
        }
        
        if col_opponent_count >= 3 {
            threats += col_opponent_count;
        }
        
        threats
    }

    // 评估潜在威胁
    fn potential_threats(board: &Board, pos: (usize, usize), opponent: Player) -> i32 {
        // 评估位置对对手的战略价值
        Self::position_value(board, pos, opponent) / 2
    }

    // 评估位置控制威胁
    fn positional_threats(board: &Board, pos: (usize, usize), opponent: Player) -> i32 {
        let (r, c) = pos;
        let mut threats = 0;
        
        // 检查位置是否连接对手的多个棋子
        let connections = Self::connection_value(board, pos, opponent);
        threats += connections;
        
        // 检查位置是否在对手的潜在模式中
        if Self::reward_participation_value(board, pos, opponent) > 0 {
            threats += 2;
        }
        
        threats
    }



    //----

    // 评估奖励模式潜力
    fn reward_potential(board: &Board, pos: (usize, usize), player: Player) -> i32 {
        let mut potential = 0;
        
        // 检查成方潜力
        for &(r, c) in &[
            (pos.0, pos.1), (pos.0, pos.1.saturating_sub(1)), 
            (pos.0.saturating_sub(1), pos.1), (pos.0.saturating_sub(1), pos.1.saturating_sub(1))
        ] {
            if r < 4 && c < 4 {
                potential += Self::square_potential(board, r, c, player);
            }
        }
        
        potential
    }

    // 评估成方潜力
    fn square_potential(board: &Board, r: usize, c: usize, player: Player) -> i32 {
        let positions = [(r, c), (r, c+1), (r+1, c), (r+1, c+1)];
        let mut player_count = 0;
        let mut empty_count = 0;
        
        for (r, c) in positions {
            match board.grid[r][c] {
                Cell::Occupied(p) if p == player => player_count += 1,
                Cell::Empty => empty_count += 1,
                _ => {}
            }
        }
        
        // 潜力评分：已有棋子越多，潜力越大
        match player_count {
            3 if empty_count == 1 => 3,  // 差一个成方
            2 if empty_count == 2 => 1,  // 两个棋子
            _ => 0
        }
    }

    // 检查位置是否是潜在威胁
    fn is_potential_threat(board: &Board, square: (usize, usize), player: Player, exclude: (usize, usize)) -> bool {
        let positions = [
            (square.0, square.1),
            (square.0, square.1 + 1),
            (square.0 + 1, square.1),
            (square.0 + 1, square.1 + 1),
        ];
        
        let mut player_count = 0;
        let mut empty_count = 0;
        let mut exclude_count = 0;
        
        for &(r, c) in &positions {
            if (r, c) == exclude {
                exclude_count += 1;
                continue;
            }
            
            match board.grid[r][c] {
                Cell::Occupied(p) if p == player => player_count += 1,
                Cell::Empty => empty_count += 1,
                _ => {}
            }
        }
        
        // 排除位置后，如果已有3个棋子且1个空位，则是威胁
        player_count == 3 && empty_count == 1 && exclude_count == 0
    }

    // 判断棋子是否可能参与奖励模式
    fn is_potential_reward_piece(board: &Board, pos: (usize, usize)) -> bool {
        let (r, c) = pos;
        
        // 中心位置更可能参与多个模式
        if r == 2 && c == 2 {
            return true;
        }
        
        // 检查是否在关键行/列
        r == 2 || c == 2
    }
}

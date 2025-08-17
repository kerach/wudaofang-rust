fn main() {
    let mut board = Board::new();
    
    // 落子阶段示例
    println!("=== 落子阶段 ===");
    let placements = [
        (0, 0), (0, 1), (1, 0), (1, 1), // 成方
        (0, 2), (1, 3), (2, 4),         // 成三斜
        (0, 3), (1, 2), (2, 1), (3, 0), // 成四斜
        (0, 4), (1, 4), (2, 4), (3, 4), (4, 4), // 成州(列)
        (4, 0), (4, 1), (4, 2), (4, 3), (4, 4), // 成州(行)
    ];
    
    for (i, (r, c)) in placements.iter().enumerate() {
        println!("落子 {}: 玩家在 ({}, {}) 落子", i + 1, r, c);
        match board.place_piece(*r, *c) {
            Ok(extra) => println!("  获得额外落子次数: {}", extra),
            Err(e) => println!("  错误: {}", e),
        }
        board.print_board();
    }
    
    // 吃棋阶段示例
    if board.phase == GamePhase::Capture {
        println!("\n=== 吃棋阶段 ===");
        
        // 后落子的玩家（白方）先吃棋
        println!("白方吃棋 (0,0)");
        match board.capture_piece(0, 0) {
            Ok(_) => println!("  吃棋成功"),
            Err(e) => println!("  错误: {}", e),
        }
        board.print_board();
        
        // 先落子的玩家（黑方）后吃棋
        println!("黑方吃棋 (4,4)");
        match board.capture_piece(4, 4) {
            Ok(_) => println!("  吃棋成功"),
            Err(e) => println!("  错误: {}", e),
        }
        board.print_board();
    }
    
    // 移动阶段示例
    if board.phase == GamePhase::Movement {
        println!("\n=== 移动阶段 ===");
        // 移动一个棋子形成成龙
        println!("移动棋子: (2, 2) -> (2, 1)");
        match board.move_piece((2, 2), (2, 1)) {
            Ok(captured) => println!("  吃掉 {} 个棋子", captured),
            Err(e) => println!("  错误: {}", e),
        }
        board.print_board();
    }
    
    // 保存和加载棋谱
    println!("\n=== 棋谱记录 ===");
    let record = board.get_game_record().clone();
    println!("棋谱包含 {} 步操作", record.len());
    
    // 序列化棋谱
    let serialized = serde_json::to_string(&record).unwrap();
    println!("序列化棋谱长度: {} 字节", serialized.len());
    
    // 反序列化棋谱
    let deserialized: Vec<GameAction> = serde_json::from_str(&serialized).unwrap();
    
    // 使用棋谱重放游戏
    println!("\n=== 棋谱重放 ===");
    let mut replayer = GameReplayer::new(deserialized);
    while replayer.step_forward().is_some() {
        println!("步骤 {}:", replayer.current_step);
        replayer.get_current_board().print_board();
    }
}
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
                let input = read_input(&format!("{} 请选择要移动的棋子: ", board.current_player));
                
                match parse_coord(&input) {
                    Ok(from) => {
                        let input = read_input(&format!("{} 请输入目标位置: ", board.current_player));
                        
                        match parse_coord(&input) {
                            Ok(to) => {
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
                    Err(e) => println!("输入错误: {}", e),
                }
            }
        }
        
        // 检查游戏是否结束
        let opponent = board.current_player.opponent();
        if board.player_pieces(opponent).len() < 3 {
            println!("\n===== 游戏结束! =====");
            println!("{} 获胜! 因为对手只剩 {} 个棋子", board.current_player, board.player_pieces(opponent).len());
            game_over = true;
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
}
pub mod core;
pub mod doro;
pub mod downloader;

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{LazyLock, Mutex};

use luo9_sdk::Bot;
use luo9_sdk::Msg;
use luo9_sdk::command::{Command, PrefixMode};
use serde_json::json;

use doro::DoroEndingManager;
use downloader::download_assets;

static DATA_DIR: LazyLock<String> = LazyLock::new(|| resolve("data/plugin_doro"));
static PIC_DIR: LazyLock<String> = LazyLock::new(|| resolve("data/plugin_doro/DoroEndingPic"));
static JSON_FILE: LazyLock<String> = LazyLock::new(|| resolve("data/plugin_doro/doroendings.json"));
static DATE_FILE: LazyLock<String> = LazyLock::new(|| resolve("data/plugin_doro/date_record.json"));
static USER_MAP_FILE: LazyLock<String> = LazyLock::new(|| resolve("data/plugin_doro/user_doro_map.json"));

static MANAGER: Mutex<Option<DoroEndingManager>> = Mutex::new(None);
static USER_MAP: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);
static CURRENT_DATE: Mutex<String> = Mutex::new(String::new());

fn resolve(rel: &str) -> String {
    std::env::current_dir().unwrap().join(rel).to_string_lossy().into_owned()
}

// ── 初始化 ──────────────────────────────────────────────────────

pub fn init_data() {
    let _ = std::fs::create_dir_all(&*DATA_DIR);
    let _ = std::fs::create_dir_all(&*PIC_DIR);

    if !std::path::Path::new(&*JSON_FILE).exists() {
        println!("[doro] 未找到本地结局数据，开始下载...");
        match download_assets(&DATA_DIR) {
            Ok(r) if r["success"].as_bool().unwrap_or(false) => {
                println!("[doro] 资源下载成功，来源: {}", r["source"].as_str().unwrap_or("?"));
            }
            Ok(r) => eprintln!("[doro] 资源下载失败: {}", r["message"].as_str().unwrap_or("?")),
            Err(e) => eprintln!("[doro] 下载出错: {}", e),
        }
    }

    let mut manager = DoroEndingManager::new(&JSON_FILE, &PIC_DIR);
    if manager.load() {
        println!("[doro] 已加载 {} 条结局", manager.count());
    }
    *MANAGER.lock().unwrap() = Some(manager);

    if let Ok(data) = load_json(&DATE_FILE) {
        if let Some(date) = data.get("date").and_then(|v| v.as_str()) {
            *CURRENT_DATE.lock().unwrap() = date.to_string();
        }
    }

    if let Ok(data) = load_json(&USER_MAP_FILE) {
        let mut guard = USER_MAP.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        if let Some(obj) = data.as_object() {
            for (k, v) in obj {
                if let Some(id) = v.as_u64() {
                    map.insert(k.clone(), id);
                }
            }
        }
        println!("[doro] 已加载 {} 条用户映射", map.len());
    }

    println!("[doro] 插件初始化完成");
}

fn load_json(path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn save_json(path: &str, value: &serde_json::Value) {
    if let Ok(s) = serde_json::to_string_pretty(value) {
        let _ = std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap());
        let _ = std::fs::write(path, s);
    }
}

// ── 消息处理 ────────────────────────────────────────────────────

pub fn handle_group_msg(group_id: u64, user_id: u64, msg: &str) {
    let uid = user_id.to_string();
    let trimmed = msg.trim();

    if trimmed == "今日doro结局" || trimmed == "doro结局帮助" {
        if trimmed == "doro结局帮助" {
            send_help(group_id);
        } else {
            send_today(group_id, &uid);
        }
        return;
    }

    if let Some(cmd) = Command::parse(msg, "doro", PrefixMode::None) {
        if !cmd.has_args() {
            send_today(group_id, &uid);
            return;
        }
        cmd.on("add", |args| cmd_add(group_id, args))
            .on("remove", |args| cmd_remove(group_id, args))
            .on("update", |args| cmd_update(group_id, args))
            .on("list", |_| cmd_list(group_id))
            .on("help", |_| send_help(group_id))
            .otherwise(|| send_today(group_id, &uid));
    }
}

// ── 今日结局 ────────────────────────────────────────────────────

fn send_today(group_id: u64, uid: &str) {
    match get_today_ending(uid) {
        Some((_id, pic)) => {
            Bot::send_group_msg(group_id, Msg::image(&format!("{}/{}", &*PIC_DIR, pic)).build());
        }
        None => { Bot::send_group_msg(group_id, CString::new("暂无doro结局数据").unwrap()); }
    }
}

fn get_today_ending(uid: &str) -> Option<(u64, String)> {
    let today = today_string();
    let mut date = CURRENT_DATE.lock().unwrap();

    if *date != today {
        println!("[doro] 日期变更: {} -> {}", *date, today);
        *USER_MAP.lock().unwrap() = None;
        *date = today.clone();
        save_json(&DATE_FILE, &json!({"date": today}));
        save_json(&USER_MAP_FILE, &json!({}));
    }

    let mut map_guard = USER_MAP.lock().unwrap();
    let map = map_guard.get_or_insert_with(HashMap::new);

    // 已有记录
    if let Some(&ending_id) = map.get(uid) {
        let mgr = MANAGER.lock().unwrap();
        if let Some(ref m) = *mgr {
            if let Some(e) = m.get_by_id(ending_id) {
                return Some((e.id, e.pic.clone()));
            }
        }
        map.remove(uid);
    }

    // 随机分配
    let mgr = MANAGER.lock().unwrap();
    let m = (*mgr).as_ref()?;
    let all = m.get_all();
    if all.is_empty() { return None; }

    let idx = random_index(all.len());
    let ending = &all[idx];
    let result = (ending.id, ending.pic.clone());

    drop(mgr);
    map.insert(uid.to_string(), result.0);
    let map_json: serde_json::Value = map.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
    save_json(&USER_MAP_FILE, &map_json);

    Some(result)
}

// ── 管理命令 ────────────────────────────────────────────────────

fn cmd_add(group_id: u64, args: &[String]) {
    if args.len() < 2 {
        Bot::send_group_msg(group_id, CString::new("用法: doro add <中文名> <英文名> [图片URL]").unwrap());
        return;
    }
    let mut mgr = MANAGER.lock().unwrap();
    if let Some(ref mut m) = *mgr {
        let r = m.add(&args[0], &args[1], args.get(2).map(|s| s.as_str()));
        match r {
            Ok(e) => reply(group_id, &format!("doro结局 '{}' 添加成功！(ID: {})", e.name, e.id)),
            Err(e) => reply(group_id, &format!("添加失败: {}", e)),
        }
    }
}

fn cmd_remove(group_id: u64, args: &[String]) {
    if args.is_empty() {
        reply(group_id, "用法: doro remove <ID或中文名>");
        return;
    }
    let mut mgr = MANAGER.lock().unwrap();
    if let Some(ref mut m) = *mgr {
        match m.remove(&args[0]) {
            Ok(true) => reply(group_id, "doro结局删除成功！"),
            Ok(false) => reply(group_id, "未找到指定的doro结局"),
            Err(e) => reply(group_id, &format!("删除失败: {}", e)),
        }
    }
}

fn cmd_update(group_id: u64, args: &[String]) {
    if args.len() < 3 {
        reply(group_id, "用法: doro update <ID> <1(中文名)|2(英文名)> <新值>");
        return;
    }
    let id: u64 = match args[0].parse() {
        Ok(n) => n,
        Err(_) => { reply(group_id, "ID 应为数字"); return; }
    };
    let field = match args[1].as_str() {
        "1" | "中文" | "中文名" => "name",
        "2" | "英文" | "英文名" | "en" => "english_name",
        _ => { reply(group_id, "无效选项，输入 1（中文名）或 2（英文名）"); return; }
    };
    let mut mgr = MANAGER.lock().unwrap();
    if let Some(ref mut m) = *mgr {
        match m.update(id, field, &args[2]) {
            Ok(e) => {
                let fc = if field == "name" { "中文名" } else { "英文名" };
                reply(group_id, &format!("修改成功！ID:{} 新{}: {}", e.id, fc, &args[2]));
            }
            Err(e) => reply(group_id, &format!("修改失败: {}", e)),
        }
    }
}

fn cmd_list(group_id: u64) {
    let mgr = MANAGER.lock().unwrap();
    if let Some(ref m) = *mgr {
        let all = m.get_all();
        if all.is_empty() {
            reply(group_id, "当前没有任何doro结局数据！");
            return;
        }
        let mut lines = vec!["所有doro结局：".to_string()];
        for e in all {
            lines.push(format!("{}. {} ({})", e.id, e.name, e.english_name));
        }
        reply(group_id, &lines.join("\n"));
    }
}

fn send_help(group_id: u64) {
    reply(group_id, "\
doro 结局指令列表：
- 今日doro结局：获取今日的 doro 结局
- doro结局帮助：查看本帮助
- doro add <中文名> <英文名> [图片URL]：添加新结局
- doro remove <ID或中文名>：删除结局
- doro update <ID> <1|2> <新值>：修改结局
- doro list：列出所有结局");
}

fn reply(group_id: u64, text: &str) {
    Bot::send_group_msg(group_id, CString::new(text).unwrap());
}

// ── 日期计算 ────────────────────────────────────────────────────

fn today_string() -> String {
    // 基于儒略日的日期计算，比逐 year 循环更高效
    let utc_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let secs = utc_secs + 8 * 3600; // UTC+8
    let days = (secs / 86400) as i64;

    // Unix epoch (1970-01-01) 的儒略日数 = 2440588
    let jd = days + 2440588;
    let (y, m, d) = jd_to_ymd(jd);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// 儒略日转年月日（Fliegel & Van Flandern 算法）
fn jd_to_ymd(jd: i64) -> (i64, u64, u64) {
    let l = jd + 68569;
    let n = 4 * l / 146097;
    let l = l - (146097 * n + 3) / 4;
    let i = 4000 * (l + 1) / 1461001;
    let l = l - 1461 * i / 4 + 31;
    let j = 80 * l / 2447;
    let d = l - 2447 * j / 80;
    let l = j / 11;
    let m = j + 2 - 12 * l;
    let y = 100 * (n - 49) + i + l;
    (y, m as u64, d as u64)
}

fn random_index(len: usize) -> usize {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64);
    (h.finish() as usize) % len
}

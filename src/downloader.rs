use serde_json::Value;
use std::fs;
use std::path::Path;

struct SourceConfig {
    api: &'static str,
    raw: &'static str,
}

const GITHUB: SourceConfig = SourceConfig {
    api: "https://api.github.com/repos/SeeWhyRan/doroending_pic_assets/contents",
    raw: "https://raw.githubusercontent.com/SeeWhyRan/doroending_pic_assets/main",
};

const GITEE: SourceConfig = SourceConfig {
    api: "https://gitee.com/api/v5/repos/seewhy_ran/doroending_pic_assets/contents",
    raw: "https://gitee.com/seewhy_ran/doroending_pic_assets/raw/main",
};

pub fn download_assets(target_dir: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let target = Path::new(target_dir);
    fs::create_dir_all(target)?;

    // 尝试 GitHub
    println!("[doro] 尝试从 GitHub 下载...");
    match try_source(&GITHUB, target) {
        Ok(result) => {
            println!("[doro] GitHub 下载成功");
            return Ok(serde_json::json!({
                "success": true,
                "message": "下载完成",
                "source": "github",
                "json_data": result
            }));
        }
        Err(e) => {
            println!("[doro] GitHub 下载失败: {}，切换到 Gitee...", e);
        }
    }

    // 回退到 Gitee
    println!("[doro] 尝试从 Gitee 下载...");
    match try_source(&GITEE, target) {
        Ok(result) => {
            println!("[doro] Gitee 下载成功");
            return Ok(serde_json::json!({
                "success": true,
                "message": "下载完成",
                "source": "gitee",
                "json_data": result
            }));
        }
        Err(e) => {
            return Ok(serde_json::json!({
                "success": false,
                "message": format!("下载失败: {}", e),
                "source": "gitee"
            }));
        }
    }
}

fn try_source(
    source: &SourceConfig,
    target: &Path,
) -> Result<Value, Box<dyn std::error::Error>> {
    // 获取根目录列表
    let root_data = request(source.api)?;
    let root: Vec<Value> = serde_json::from_str(&root_data)?;

    let has_pic = root.iter().any(|i| {
        i["name"].as_str() == Some("DoroEndingPic") && i["type"].as_str() == Some("dir")
    });
    let has_json = root.iter().any(|i| i["name"].as_str() == Some("doroendings.json"));

    // 下载图片目录
    if has_pic {
        download_dir(source, "DoroEndingPic", &target.join("DoroEndingPic"))?;
    }

    // 下载 JSON 文件
    let mut json_data: Option<Value> = None;
    if has_json {
        let json_path = target.join("doroendings.json");
        let raw_url = format!("{}/doroendings.json", source.raw);
        if download_file(&raw_url, &json_path)? {
            let content = fs::read_to_string(&json_path)?;
            json_data = Some(serde_json::from_str(&content)?);
        }
    }

    // 验证
    if has_json && !target.join("doroendings.json").exists() {
        return Err("JSON 文件下载失败".into());
    }
    if has_pic && !target.join("DoroEndingPic").exists() {
        return Err("图片目录下载失败".into());
    }

    Ok(json_data.unwrap_or(Value::Null))
}

fn download_dir(
    source: &SourceConfig,
    api_path: &str,
    local_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/{}", source.api, api_path);
    let data = request(&url)?;
    let items: Vec<Value> = serde_json::from_str(&data)?;

    fs::create_dir_all(local_dir)?;

    for item in &items {
        let name = item["name"].as_str().unwrap_or("");
        let item_type = item["type"].as_str().unwrap_or("");
        let has_download = item.get("download_url").is_some();

        if item_type == "file" || has_download {
            let raw_url = format!("{}/{}/{}", source.raw, api_path, name);
            let save_path = local_dir.join(name);
            download_file(&raw_url, &save_path)?;
        } else if item_type == "dir" {
            let sub_api = format!("{}/{}", api_path, name);
            download_dir(source, &sub_api, &local_dir.join(name))?;
        }
    }

    Ok(())
}

fn download_file(url: &str, save_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    // 已存在且 >100B 则跳过
    if save_path.exists() && save_path.metadata()?.len() > 100 {
        return Ok(true);
    }

    let data = match request(url) {
        Ok(d) => d.into_bytes(),
        Err(_) => return Ok(false),
    };

    fs::create_dir_all(save_path.parent().unwrap())?;
    fs::write(save_path, &data)?;
    Ok(true)
}

fn request(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut resp = ureq::get(url)
        .header("User-Agent", "DoroDownloader/2.0")
        .call()
        .map_err(|e| format!("HTTP 请求失败: {}", e))?;

    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {}", e))?;

    Ok(body)
}

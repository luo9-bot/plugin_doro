use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoroEnding {
    pub id: u64,
    pub name: String,
    pub english_name: String,
    #[serde(default)]
    pub pic: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DoroData {
    #[serde(default)]
    datas: Vec<DoroEnding>,
    #[serde(default)]
    max_id: u64,
    #[serde(default)]
    total: usize,
}

pub struct DoroEndingManager {
    data_file: String,
    pic_dir: String,
    endings: Vec<DoroEnding>,
    max_id: u64,
}

impl DoroEndingManager {
    pub fn new(data_file: &str, pic_dir: &str) -> Self {
        Self {
            data_file: data_file.to_string(),
            pic_dir: pic_dir.to_string(),
            endings: Vec::new(),
            max_id: 0,
        }
    }

    pub fn load(&mut self) -> bool {
        let content = match std::fs::read_to_string(&self.data_file) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("[doro] 数据文件不存在: {}", self.data_file);
                return false;
            }
        };

        let data: DoroData = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[doro] 解析失败: {}", e);
                return false;
            }
        };

        self.max_id = data.max_id;
        self.endings = data.datas;
        println!("[doro] 已加载 {} 条结局", self.endings.len());
        true
    }

    fn save(&self) -> Result<(), String> {
        let data = DoroData {
            datas: self.endings.clone(),
            max_id: self.max_id,
            total: self.endings.len(),
        };
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(std::path::Path::new(&self.data_file).parent().unwrap())
            .map_err(|e| e.to_string())?;
        std::fs::write(&self.data_file, json).map_err(|e| e.to_string())
    }

    pub fn get_all(&self) -> &[DoroEnding] {
        &self.endings
    }

    pub fn count(&self) -> usize {
        self.endings.len()
    }

    pub fn get_by_id(&self, id: u64) -> Option<&DoroEnding> {
        self.endings.iter().find(|e| e.id == id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&DoroEnding> {
        self.endings.iter().find(|e| e.name == name)
    }

    pub fn add(
        &mut self,
        name: &str,
        english_name: &str,
        image_url: Option<&str>,
    ) -> Result<DoroEnding, String> {
        if self.endings.iter().any(|e| e.english_name == english_name) {
            return Err(format!("英文名 '{}' 已存在", english_name));
        }

        let new_id = self.max_id + 1;
        let mut pic_name = String::new();

        if let Some(url) = image_url {
            let safe_name: String = english_name
                .chars()
                .map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
                .take(50)
                .collect();
            let filename = format!("{:08}_{}", new_id, safe_name);

            match download_image(url, &self.pic_dir, &filename) {
                Ok(saved_name) => pic_name = saved_name,
                Err(e) => return Err(format!("图片保存失败: {}", e)),
            }
        }

        let ending = DoroEnding {
            id: new_id,
            name: name.to_string(),
            english_name: english_name.to_string(),
            pic: pic_name,
        };

        self.endings.push(ending.clone());
        self.max_id = new_id;
        self.save()?;
        println!("[doro] 添加结局: {} (ID: {})", name, new_id);
        Ok(ending)
    }

    pub fn remove(&mut self, target: &str) -> Result<bool, String> {
        let idx = self.endings.iter().position(|e| {
            e.id.to_string() == target || e.name == target
        });

        let idx = match idx {
            Some(i) => i,
            None => return Ok(false),
        };

        let ending = self.endings.remove(idx);

        if !ending.pic.is_empty() {
            let img_path = format!("{}/{}", self.pic_dir, ending.pic);
            let _ = std::fs::remove_file(&img_path);
        }

        if ending.id == self.max_id {
            self.max_id = self.endings.iter().map(|e| e.id).max().unwrap_or(0);
        }

        self.save()?;
        println!("[doro] 删除结局: {} (ID: {})", ending.name, ending.id);
        Ok(true)
    }

    pub fn update(
        &mut self,
        id: u64,
        field: &str,
        value: &str,
    ) -> Result<DoroEnding, String> {
        if !self.endings.iter().any(|e| e.id == id) {
            return Err(format!("未找到 ID 为 {} 的结局", id));
        }

        if field == "english_name" && self.endings.iter().any(|e| e.id != id && e.english_name == value) {
            return Err(format!("英文名 '{}' 已存在", value));
        }

        let ending = self.endings.iter_mut().find(|e| e.id == id).unwrap();

        match field {
            "name" => ending.name = value.to_string(),
            "english_name" => ending.english_name = value.to_string(),
            _ => return Err("未知字段".to_string()),
        }

        let result = ending.clone();
        self.save()?;
        println!("[doro] 更新结局: {} (ID: {})", result.name, result.id);
        Ok(result)
    }
}

fn download_image(url: &str, pic_dir: &str, base_name: &str) -> Result<String, String> {
    let mut resp = ureq::get(url)
        .header("User-Agent", "DoroDownloader/1.0")
        .call()
        .map_err(|e| format!("HTTP 请求失败: {}", e))?;

    let data = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("读取响应失败: {}", e))?;

    if data.len() > 10 * 1024 * 1024 {
        return Err("图片过大 (超过10MB)".to_string());
    }

    let ext = detect_image_ext(&data);
    let filename = format!("{}{}", base_name, ext);
    let path = format!("{}/{}", pic_dir, filename);

    std::fs::create_dir_all(pic_dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;

    Ok(filename)
}

fn detect_image_ext(data: &[u8]) -> &str {
    if data.len() < 4 {
        return ".jpg";
    }
    if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return ".jpg";
    }
    if data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
        return ".png";
    }
    if data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x38 {
        return ".gif";
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return ".webp";
    }
    if data[0] == 0x42 && data[1] == 0x4D {
        return ".bmp";
    }
    ".jpg"
}

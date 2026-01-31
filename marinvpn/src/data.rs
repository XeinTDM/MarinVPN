use crate::models::{Region, City};

pub fn get_default_regions() -> Vec<Region> {
    vec![
        Region {
            name: "Sweden".to_string(),
            flag: "🇸🇪".to_string(),
            map_x: 460.0,
            map_y: 140.0,
            cities: vec![
                City { name: "Stockholm".to_string(), load: 45, ping: 12 },
                City { name: "Gothenburg".to_string(), load: 22, ping: 14 },
                City { name: "Malmö".to_string(), load: 89, ping: 15 },
            ],
        },
        Region {
            name: "United States".to_string(),
            flag: "🇺🇸".to_string(),
            map_x: 230.0,
            map_y: 200.0,
            cities: vec![
                City { name: "New York".to_string(), load: 92, ping: 110 },
                City { name: "Los Angeles".to_string(), load: 65, ping: 150 },
                City { name: "Chicago".to_string(), load: 30, ping: 130 },
                City { name: "Dallas".to_string(), load: 12, ping: 140 },
                City { name: "Miami".to_string(), load: 45, ping: 120 },
            ],
        },
        Region {
            name: "Germany".to_string(),
            flag: "🇩🇪".to_string(),
            map_x: 450.0,
            map_y: 170.0,
            cities: vec![
                City { name: "Frankfurt".to_string(), load: 78, ping: 25 },
                City { name: "Berlin".to_string(), load: 55, ping: 28 },
                City { name: "Munich".to_string(), load: 33, ping: 30 },
            ],
        },
        Region {
            name: "United Kingdom".to_string(),
            flag: "🇬🇧".to_string(),
            map_x: 420.0,
            map_y: 160.0,
            cities: vec![
                City { name: "London".to_string(), load: 95, ping: 35 },
                City { name: "Manchester".to_string(), load: 40, ping: 38 },
            ],
        },
        Region {
            name: "Netherlands".to_string(),
            flag: "🇳🇱".to_string(),
            map_x: 440.0,
            map_y: 165.0,
            cities: vec![
                City { name: "Amsterdam".to_string(), load: 82, ping: 18 },
                City { name: "Rotterdam".to_string(), load: 25, ping: 20 },
            ],
        },
    ]
}
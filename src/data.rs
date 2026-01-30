use crate::models::{Region, City};

pub const REGIONS: &[Region] = &[
    Region {
        name: "Sweden",
        flag: "🇸🇪",
        map_x: 460.0,
        map_y: 140.0,
        cities: &[
            City { name: "Stockholm", load: 45, ping: 12 },
            City { name: "Gothenburg", load: 22, ping: 14 },
            City { name: "Malmö", load: 89, ping: 15 },
        ],
    },
    Region {
        name: "United States",
        flag: "🇺🇸",
        map_x: 230.0,
        map_y: 200.0,
        cities: &[
            City { name: "New York", load: 92, ping: 110 },
            City { name: "Los Angeles", load: 65, ping: 150 },
            City { name: "Chicago", load: 30, ping: 130 },
            City { name: "Dallas", load: 12, ping: 140 },
            City { name: "Miami", load: 45, ping: 120 },
        ],
    },
    Region {
        name: "Germany",
        flag: "🇩🇪",
        map_x: 450.0,
        map_y: 170.0,
        cities: &[
            City { name: "Frankfurt", load: 78, ping: 25 },
            City { name: "Berlin", load: 55, ping: 28 },
            City { name: "Munich", load: 33, ping: 30 },
        ],
    },
    Region {
        name: "United Kingdom",
        flag: "🇬🇧",
        map_x: 420.0,
        map_y: 160.0,
        cities: &[
            City { name: "London", load: 95, ping: 35 },
            City { name: "Manchester", load: 40, ping: 38 },
        ],
    },
    Region {
        name: "Netherlands",
        flag: "🇳🇱",
        map_x: 440.0,
        map_y: 165.0,
        cities: &[
            City { name: "Amsterdam", load: 82, ping: 18 },
            City { name: "Rotterdam", load: 25, ping: 20 },
        ],
    },
];

pub fn get_regions() -> &'static [Region] {
    REGIONS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeZoneInfo {
    pub id: &'static str,
    pub member: &'static str,
}

pub const TZDB_VERSION: &str = "2026c";
pub const TIME_ZONE_COUNT: usize = 598;

pub static TIME_ZONES: [TimeZoneInfo; TIME_ZONE_COUNT] = [
    TimeZoneInfo {
        id: "Africa/Abidjan",
        member: "Africa_Abidjan",
    },
    TimeZoneInfo {
        id: "Africa/Accra",
        member: "Africa_Accra",
    },
    TimeZoneInfo {
        id: "Africa/Addis_Ababa",
        member: "Africa_Addis_Ababa",
    },
    TimeZoneInfo {
        id: "Africa/Algiers",
        member: "Africa_Algiers",
    },
    TimeZoneInfo {
        id: "Africa/Asmara",
        member: "Africa_Asmara",
    },
    TimeZoneInfo {
        id: "Africa/Asmera",
        member: "Africa_Asmera",
    },
    TimeZoneInfo {
        id: "Africa/Bamako",
        member: "Africa_Bamako",
    },
    TimeZoneInfo {
        id: "Africa/Bangui",
        member: "Africa_Bangui",
    },
    TimeZoneInfo {
        id: "Africa/Banjul",
        member: "Africa_Banjul",
    },
    TimeZoneInfo {
        id: "Africa/Bissau",
        member: "Africa_Bissau",
    },
    TimeZoneInfo {
        id: "Africa/Blantyre",
        member: "Africa_Blantyre",
    },
    TimeZoneInfo {
        id: "Africa/Brazzaville",
        member: "Africa_Brazzaville",
    },
    TimeZoneInfo {
        id: "Africa/Bujumbura",
        member: "Africa_Bujumbura",
    },
    TimeZoneInfo {
        id: "Africa/Cairo",
        member: "Africa_Cairo",
    },
    TimeZoneInfo {
        id: "Africa/Casablanca",
        member: "Africa_Casablanca",
    },
    TimeZoneInfo {
        id: "Africa/Ceuta",
        member: "Africa_Ceuta",
    },
    TimeZoneInfo {
        id: "Africa/Conakry",
        member: "Africa_Conakry",
    },
    TimeZoneInfo {
        id: "Africa/Dakar",
        member: "Africa_Dakar",
    },
    TimeZoneInfo {
        id: "Africa/Dar_es_Salaam",
        member: "Africa_Dar_es_Salaam",
    },
    TimeZoneInfo {
        id: "Africa/Djibouti",
        member: "Africa_Djibouti",
    },
    TimeZoneInfo {
        id: "Africa/Douala",
        member: "Africa_Douala",
    },
    TimeZoneInfo {
        id: "Africa/El_Aaiun",
        member: "Africa_El_Aaiun",
    },
    TimeZoneInfo {
        id: "Africa/Freetown",
        member: "Africa_Freetown",
    },
    TimeZoneInfo {
        id: "Africa/Gaborone",
        member: "Africa_Gaborone",
    },
    TimeZoneInfo {
        id: "Africa/Harare",
        member: "Africa_Harare",
    },
    TimeZoneInfo {
        id: "Africa/Johannesburg",
        member: "Africa_Johannesburg",
    },
    TimeZoneInfo {
        id: "Africa/Juba",
        member: "Africa_Juba",
    },
    TimeZoneInfo {
        id: "Africa/Kampala",
        member: "Africa_Kampala",
    },
    TimeZoneInfo {
        id: "Africa/Khartoum",
        member: "Africa_Khartoum",
    },
    TimeZoneInfo {
        id: "Africa/Kigali",
        member: "Africa_Kigali",
    },
    TimeZoneInfo {
        id: "Africa/Kinshasa",
        member: "Africa_Kinshasa",
    },
    TimeZoneInfo {
        id: "Africa/Lagos",
        member: "Africa_Lagos",
    },
    TimeZoneInfo {
        id: "Africa/Libreville",
        member: "Africa_Libreville",
    },
    TimeZoneInfo {
        id: "Africa/Lome",
        member: "Africa_Lome",
    },
    TimeZoneInfo {
        id: "Africa/Luanda",
        member: "Africa_Luanda",
    },
    TimeZoneInfo {
        id: "Africa/Lubumbashi",
        member: "Africa_Lubumbashi",
    },
    TimeZoneInfo {
        id: "Africa/Lusaka",
        member: "Africa_Lusaka",
    },
    TimeZoneInfo {
        id: "Africa/Malabo",
        member: "Africa_Malabo",
    },
    TimeZoneInfo {
        id: "Africa/Maputo",
        member: "Africa_Maputo",
    },
    TimeZoneInfo {
        id: "Africa/Maseru",
        member: "Africa_Maseru",
    },
    TimeZoneInfo {
        id: "Africa/Mbabane",
        member: "Africa_Mbabane",
    },
    TimeZoneInfo {
        id: "Africa/Mogadishu",
        member: "Africa_Mogadishu",
    },
    TimeZoneInfo {
        id: "Africa/Monrovia",
        member: "Africa_Monrovia",
    },
    TimeZoneInfo {
        id: "Africa/Nairobi",
        member: "Africa_Nairobi",
    },
    TimeZoneInfo {
        id: "Africa/Ndjamena",
        member: "Africa_Ndjamena",
    },
    TimeZoneInfo {
        id: "Africa/Niamey",
        member: "Africa_Niamey",
    },
    TimeZoneInfo {
        id: "Africa/Nouakchott",
        member: "Africa_Nouakchott",
    },
    TimeZoneInfo {
        id: "Africa/Ouagadougou",
        member: "Africa_Ouagadougou",
    },
    TimeZoneInfo {
        id: "Africa/Porto-Novo",
        member: "Africa_Porto_Minus_Novo",
    },
    TimeZoneInfo {
        id: "Africa/Sao_Tome",
        member: "Africa_Sao_Tome",
    },
    TimeZoneInfo {
        id: "Africa/Timbuktu",
        member: "Africa_Timbuktu",
    },
    TimeZoneInfo {
        id: "Africa/Tripoli",
        member: "Africa_Tripoli",
    },
    TimeZoneInfo {
        id: "Africa/Tunis",
        member: "Africa_Tunis",
    },
    TimeZoneInfo {
        id: "Africa/Windhoek",
        member: "Africa_Windhoek",
    },
    TimeZoneInfo {
        id: "America/Adak",
        member: "America_Adak",
    },
    TimeZoneInfo {
        id: "America/Anchorage",
        member: "America_Anchorage",
    },
    TimeZoneInfo {
        id: "America/Anguilla",
        member: "America_Anguilla",
    },
    TimeZoneInfo {
        id: "America/Antigua",
        member: "America_Antigua",
    },
    TimeZoneInfo {
        id: "America/Araguaina",
        member: "America_Araguaina",
    },
    TimeZoneInfo {
        id: "America/Argentina/Buenos_Aires",
        member: "America_Argentina_Buenos_Aires",
    },
    TimeZoneInfo {
        id: "America/Argentina/Catamarca",
        member: "America_Argentina_Catamarca",
    },
    TimeZoneInfo {
        id: "America/Argentina/ComodRivadavia",
        member: "America_Argentina_ComodRivadavia",
    },
    TimeZoneInfo {
        id: "America/Argentina/Cordoba",
        member: "America_Argentina_Cordoba",
    },
    TimeZoneInfo {
        id: "America/Argentina/Jujuy",
        member: "America_Argentina_Jujuy",
    },
    TimeZoneInfo {
        id: "America/Argentina/La_Rioja",
        member: "America_Argentina_La_Rioja",
    },
    TimeZoneInfo {
        id: "America/Argentina/Mendoza",
        member: "America_Argentina_Mendoza",
    },
    TimeZoneInfo {
        id: "America/Argentina/Rio_Gallegos",
        member: "America_Argentina_Rio_Gallegos",
    },
    TimeZoneInfo {
        id: "America/Argentina/Salta",
        member: "America_Argentina_Salta",
    },
    TimeZoneInfo {
        id: "America/Argentina/San_Juan",
        member: "America_Argentina_San_Juan",
    },
    TimeZoneInfo {
        id: "America/Argentina/San_Luis",
        member: "America_Argentina_San_Luis",
    },
    TimeZoneInfo {
        id: "America/Argentina/Tucuman",
        member: "America_Argentina_Tucuman",
    },
    TimeZoneInfo {
        id: "America/Argentina/Ushuaia",
        member: "America_Argentina_Ushuaia",
    },
    TimeZoneInfo {
        id: "America/Aruba",
        member: "America_Aruba",
    },
    TimeZoneInfo {
        id: "America/Asuncion",
        member: "America_Asuncion",
    },
    TimeZoneInfo {
        id: "America/Atikokan",
        member: "America_Atikokan",
    },
    TimeZoneInfo {
        id: "America/Atka",
        member: "America_Atka",
    },
    TimeZoneInfo {
        id: "America/Bahia",
        member: "America_Bahia",
    },
    TimeZoneInfo {
        id: "America/Bahia_Banderas",
        member: "America_Bahia_Banderas",
    },
    TimeZoneInfo {
        id: "America/Barbados",
        member: "America_Barbados",
    },
    TimeZoneInfo {
        id: "America/Belem",
        member: "America_Belem",
    },
    TimeZoneInfo {
        id: "America/Belize",
        member: "America_Belize",
    },
    TimeZoneInfo {
        id: "America/Blanc-Sablon",
        member: "America_Blanc_Minus_Sablon",
    },
    TimeZoneInfo {
        id: "America/Boa_Vista",
        member: "America_Boa_Vista",
    },
    TimeZoneInfo {
        id: "America/Bogota",
        member: "America_Bogota",
    },
    TimeZoneInfo {
        id: "America/Boise",
        member: "America_Boise",
    },
    TimeZoneInfo {
        id: "America/Buenos_Aires",
        member: "America_Buenos_Aires",
    },
    TimeZoneInfo {
        id: "America/Cambridge_Bay",
        member: "America_Cambridge_Bay",
    },
    TimeZoneInfo {
        id: "America/Campo_Grande",
        member: "America_Campo_Grande",
    },
    TimeZoneInfo {
        id: "America/Cancun",
        member: "America_Cancun",
    },
    TimeZoneInfo {
        id: "America/Caracas",
        member: "America_Caracas",
    },
    TimeZoneInfo {
        id: "America/Catamarca",
        member: "America_Catamarca",
    },
    TimeZoneInfo {
        id: "America/Cayenne",
        member: "America_Cayenne",
    },
    TimeZoneInfo {
        id: "America/Cayman",
        member: "America_Cayman",
    },
    TimeZoneInfo {
        id: "America/Chicago",
        member: "America_Chicago",
    },
    TimeZoneInfo {
        id: "America/Chihuahua",
        member: "America_Chihuahua",
    },
    TimeZoneInfo {
        id: "America/Ciudad_Juarez",
        member: "America_Ciudad_Juarez",
    },
    TimeZoneInfo {
        id: "America/Coral_Harbour",
        member: "America_Coral_Harbour",
    },
    TimeZoneInfo {
        id: "America/Cordoba",
        member: "America_Cordoba",
    },
    TimeZoneInfo {
        id: "America/Costa_Rica",
        member: "America_Costa_Rica",
    },
    TimeZoneInfo {
        id: "America/Coyhaique",
        member: "America_Coyhaique",
    },
    TimeZoneInfo {
        id: "America/Creston",
        member: "America_Creston",
    },
    TimeZoneInfo {
        id: "America/Cuiaba",
        member: "America_Cuiaba",
    },
    TimeZoneInfo {
        id: "America/Curacao",
        member: "America_Curacao",
    },
    TimeZoneInfo {
        id: "America/Danmarkshavn",
        member: "America_Danmarkshavn",
    },
    TimeZoneInfo {
        id: "America/Dawson",
        member: "America_Dawson",
    },
    TimeZoneInfo {
        id: "America/Dawson_Creek",
        member: "America_Dawson_Creek",
    },
    TimeZoneInfo {
        id: "America/Denver",
        member: "America_Denver",
    },
    TimeZoneInfo {
        id: "America/Detroit",
        member: "America_Detroit",
    },
    TimeZoneInfo {
        id: "America/Dominica",
        member: "America_Dominica",
    },
    TimeZoneInfo {
        id: "America/Edmonton",
        member: "America_Edmonton",
    },
    TimeZoneInfo {
        id: "America/Eirunepe",
        member: "America_Eirunepe",
    },
    TimeZoneInfo {
        id: "America/El_Salvador",
        member: "America_El_Salvador",
    },
    TimeZoneInfo {
        id: "America/Ensenada",
        member: "America_Ensenada",
    },
    TimeZoneInfo {
        id: "America/Fort_Nelson",
        member: "America_Fort_Nelson",
    },
    TimeZoneInfo {
        id: "America/Fort_Wayne",
        member: "America_Fort_Wayne",
    },
    TimeZoneInfo {
        id: "America/Fortaleza",
        member: "America_Fortaleza",
    },
    TimeZoneInfo {
        id: "America/Glace_Bay",
        member: "America_Glace_Bay",
    },
    TimeZoneInfo {
        id: "America/Godthab",
        member: "America_Godthab",
    },
    TimeZoneInfo {
        id: "America/Goose_Bay",
        member: "America_Goose_Bay",
    },
    TimeZoneInfo {
        id: "America/Grand_Turk",
        member: "America_Grand_Turk",
    },
    TimeZoneInfo {
        id: "America/Grenada",
        member: "America_Grenada",
    },
    TimeZoneInfo {
        id: "America/Guadeloupe",
        member: "America_Guadeloupe",
    },
    TimeZoneInfo {
        id: "America/Guatemala",
        member: "America_Guatemala",
    },
    TimeZoneInfo {
        id: "America/Guayaquil",
        member: "America_Guayaquil",
    },
    TimeZoneInfo {
        id: "America/Guyana",
        member: "America_Guyana",
    },
    TimeZoneInfo {
        id: "America/Halifax",
        member: "America_Halifax",
    },
    TimeZoneInfo {
        id: "America/Havana",
        member: "America_Havana",
    },
    TimeZoneInfo {
        id: "America/Hermosillo",
        member: "America_Hermosillo",
    },
    TimeZoneInfo {
        id: "America/Indiana/Indianapolis",
        member: "America_Indiana_Indianapolis",
    },
    TimeZoneInfo {
        id: "America/Indiana/Knox",
        member: "America_Indiana_Knox",
    },
    TimeZoneInfo {
        id: "America/Indiana/Marengo",
        member: "America_Indiana_Marengo",
    },
    TimeZoneInfo {
        id: "America/Indiana/Petersburg",
        member: "America_Indiana_Petersburg",
    },
    TimeZoneInfo {
        id: "America/Indiana/Tell_City",
        member: "America_Indiana_Tell_City",
    },
    TimeZoneInfo {
        id: "America/Indiana/Vevay",
        member: "America_Indiana_Vevay",
    },
    TimeZoneInfo {
        id: "America/Indiana/Vincennes",
        member: "America_Indiana_Vincennes",
    },
    TimeZoneInfo {
        id: "America/Indiana/Winamac",
        member: "America_Indiana_Winamac",
    },
    TimeZoneInfo {
        id: "America/Indianapolis",
        member: "America_Indianapolis",
    },
    TimeZoneInfo {
        id: "America/Inuvik",
        member: "America_Inuvik",
    },
    TimeZoneInfo {
        id: "America/Iqaluit",
        member: "America_Iqaluit",
    },
    TimeZoneInfo {
        id: "America/Jamaica",
        member: "America_Jamaica",
    },
    TimeZoneInfo {
        id: "America/Jujuy",
        member: "America_Jujuy",
    },
    TimeZoneInfo {
        id: "America/Juneau",
        member: "America_Juneau",
    },
    TimeZoneInfo {
        id: "America/Kentucky/Louisville",
        member: "America_Kentucky_Louisville",
    },
    TimeZoneInfo {
        id: "America/Kentucky/Monticello",
        member: "America_Kentucky_Monticello",
    },
    TimeZoneInfo {
        id: "America/Knox_IN",
        member: "America_Knox_IN",
    },
    TimeZoneInfo {
        id: "America/Kralendijk",
        member: "America_Kralendijk",
    },
    TimeZoneInfo {
        id: "America/La_Paz",
        member: "America_La_Paz",
    },
    TimeZoneInfo {
        id: "America/Lima",
        member: "America_Lima",
    },
    TimeZoneInfo {
        id: "America/Los_Angeles",
        member: "America_Los_Angeles",
    },
    TimeZoneInfo {
        id: "America/Louisville",
        member: "America_Louisville",
    },
    TimeZoneInfo {
        id: "America/Lower_Princes",
        member: "America_Lower_Princes",
    },
    TimeZoneInfo {
        id: "America/Maceio",
        member: "America_Maceio",
    },
    TimeZoneInfo {
        id: "America/Managua",
        member: "America_Managua",
    },
    TimeZoneInfo {
        id: "America/Manaus",
        member: "America_Manaus",
    },
    TimeZoneInfo {
        id: "America/Marigot",
        member: "America_Marigot",
    },
    TimeZoneInfo {
        id: "America/Martinique",
        member: "America_Martinique",
    },
    TimeZoneInfo {
        id: "America/Matamoros",
        member: "America_Matamoros",
    },
    TimeZoneInfo {
        id: "America/Mazatlan",
        member: "America_Mazatlan",
    },
    TimeZoneInfo {
        id: "America/Mendoza",
        member: "America_Mendoza",
    },
    TimeZoneInfo {
        id: "America/Menominee",
        member: "America_Menominee",
    },
    TimeZoneInfo {
        id: "America/Merida",
        member: "America_Merida",
    },
    TimeZoneInfo {
        id: "America/Metlakatla",
        member: "America_Metlakatla",
    },
    TimeZoneInfo {
        id: "America/Mexico_City",
        member: "America_Mexico_City",
    },
    TimeZoneInfo {
        id: "America/Miquelon",
        member: "America_Miquelon",
    },
    TimeZoneInfo {
        id: "America/Moncton",
        member: "America_Moncton",
    },
    TimeZoneInfo {
        id: "America/Monterrey",
        member: "America_Monterrey",
    },
    TimeZoneInfo {
        id: "America/Montevideo",
        member: "America_Montevideo",
    },
    TimeZoneInfo {
        id: "America/Montreal",
        member: "America_Montreal",
    },
    TimeZoneInfo {
        id: "America/Montserrat",
        member: "America_Montserrat",
    },
    TimeZoneInfo {
        id: "America/Nassau",
        member: "America_Nassau",
    },
    TimeZoneInfo {
        id: "America/New_York",
        member: "America_New_York",
    },
    TimeZoneInfo {
        id: "America/Nipigon",
        member: "America_Nipigon",
    },
    TimeZoneInfo {
        id: "America/Nome",
        member: "America_Nome",
    },
    TimeZoneInfo {
        id: "America/Noronha",
        member: "America_Noronha",
    },
    TimeZoneInfo {
        id: "America/North_Dakota/Beulah",
        member: "America_North_Dakota_Beulah",
    },
    TimeZoneInfo {
        id: "America/North_Dakota/Center",
        member: "America_North_Dakota_Center",
    },
    TimeZoneInfo {
        id: "America/North_Dakota/New_Salem",
        member: "America_North_Dakota_New_Salem",
    },
    TimeZoneInfo {
        id: "America/Nuuk",
        member: "America_Nuuk",
    },
    TimeZoneInfo {
        id: "America/Ojinaga",
        member: "America_Ojinaga",
    },
    TimeZoneInfo {
        id: "America/Panama",
        member: "America_Panama",
    },
    TimeZoneInfo {
        id: "America/Pangnirtung",
        member: "America_Pangnirtung",
    },
    TimeZoneInfo {
        id: "America/Paramaribo",
        member: "America_Paramaribo",
    },
    TimeZoneInfo {
        id: "America/Phoenix",
        member: "America_Phoenix",
    },
    TimeZoneInfo {
        id: "America/Port-au-Prince",
        member: "America_Port_Minus_au_Minus_Prince",
    },
    TimeZoneInfo {
        id: "America/Port_of_Spain",
        member: "America_Port_of_Spain",
    },
    TimeZoneInfo {
        id: "America/Porto_Acre",
        member: "America_Porto_Acre",
    },
    TimeZoneInfo {
        id: "America/Porto_Velho",
        member: "America_Porto_Velho",
    },
    TimeZoneInfo {
        id: "America/Puerto_Rico",
        member: "America_Puerto_Rico",
    },
    TimeZoneInfo {
        id: "America/Punta_Arenas",
        member: "America_Punta_Arenas",
    },
    TimeZoneInfo {
        id: "America/Rainy_River",
        member: "America_Rainy_River",
    },
    TimeZoneInfo {
        id: "America/Rankin_Inlet",
        member: "America_Rankin_Inlet",
    },
    TimeZoneInfo {
        id: "America/Recife",
        member: "America_Recife",
    },
    TimeZoneInfo {
        id: "America/Regina",
        member: "America_Regina",
    },
    TimeZoneInfo {
        id: "America/Resolute",
        member: "America_Resolute",
    },
    TimeZoneInfo {
        id: "America/Rio_Branco",
        member: "America_Rio_Branco",
    },
    TimeZoneInfo {
        id: "America/Rosario",
        member: "America_Rosario",
    },
    TimeZoneInfo {
        id: "America/Santa_Isabel",
        member: "America_Santa_Isabel",
    },
    TimeZoneInfo {
        id: "America/Santarem",
        member: "America_Santarem",
    },
    TimeZoneInfo {
        id: "America/Santiago",
        member: "America_Santiago",
    },
    TimeZoneInfo {
        id: "America/Santo_Domingo",
        member: "America_Santo_Domingo",
    },
    TimeZoneInfo {
        id: "America/Sao_Paulo",
        member: "America_Sao_Paulo",
    },
    TimeZoneInfo {
        id: "America/Scoresbysund",
        member: "America_Scoresbysund",
    },
    TimeZoneInfo {
        id: "America/Shiprock",
        member: "America_Shiprock",
    },
    TimeZoneInfo {
        id: "America/Sitka",
        member: "America_Sitka",
    },
    TimeZoneInfo {
        id: "America/St_Barthelemy",
        member: "America_St_Barthelemy",
    },
    TimeZoneInfo {
        id: "America/St_Johns",
        member: "America_St_Johns",
    },
    TimeZoneInfo {
        id: "America/St_Kitts",
        member: "America_St_Kitts",
    },
    TimeZoneInfo {
        id: "America/St_Lucia",
        member: "America_St_Lucia",
    },
    TimeZoneInfo {
        id: "America/St_Thomas",
        member: "America_St_Thomas",
    },
    TimeZoneInfo {
        id: "America/St_Vincent",
        member: "America_St_Vincent",
    },
    TimeZoneInfo {
        id: "America/Swift_Current",
        member: "America_Swift_Current",
    },
    TimeZoneInfo {
        id: "America/Tegucigalpa",
        member: "America_Tegucigalpa",
    },
    TimeZoneInfo {
        id: "America/Thule",
        member: "America_Thule",
    },
    TimeZoneInfo {
        id: "America/Thunder_Bay",
        member: "America_Thunder_Bay",
    },
    TimeZoneInfo {
        id: "America/Tijuana",
        member: "America_Tijuana",
    },
    TimeZoneInfo {
        id: "America/Toronto",
        member: "America_Toronto",
    },
    TimeZoneInfo {
        id: "America/Tortola",
        member: "America_Tortola",
    },
    TimeZoneInfo {
        id: "America/Vancouver",
        member: "America_Vancouver",
    },
    TimeZoneInfo {
        id: "America/Virgin",
        member: "America_Virgin",
    },
    TimeZoneInfo {
        id: "America/Whitehorse",
        member: "America_Whitehorse",
    },
    TimeZoneInfo {
        id: "America/Winnipeg",
        member: "America_Winnipeg",
    },
    TimeZoneInfo {
        id: "America/Yakutat",
        member: "America_Yakutat",
    },
    TimeZoneInfo {
        id: "America/Yellowknife",
        member: "America_Yellowknife",
    },
    TimeZoneInfo {
        id: "Antarctica/Casey",
        member: "Antarctica_Casey",
    },
    TimeZoneInfo {
        id: "Antarctica/Davis",
        member: "Antarctica_Davis",
    },
    TimeZoneInfo {
        id: "Antarctica/DumontDUrville",
        member: "Antarctica_DumontDUrville",
    },
    TimeZoneInfo {
        id: "Antarctica/Macquarie",
        member: "Antarctica_Macquarie",
    },
    TimeZoneInfo {
        id: "Antarctica/Mawson",
        member: "Antarctica_Mawson",
    },
    TimeZoneInfo {
        id: "Antarctica/McMurdo",
        member: "Antarctica_McMurdo",
    },
    TimeZoneInfo {
        id: "Antarctica/Palmer",
        member: "Antarctica_Palmer",
    },
    TimeZoneInfo {
        id: "Antarctica/Rothera",
        member: "Antarctica_Rothera",
    },
    TimeZoneInfo {
        id: "Antarctica/South_Pole",
        member: "Antarctica_South_Pole",
    },
    TimeZoneInfo {
        id: "Antarctica/Syowa",
        member: "Antarctica_Syowa",
    },
    TimeZoneInfo {
        id: "Antarctica/Troll",
        member: "Antarctica_Troll",
    },
    TimeZoneInfo {
        id: "Antarctica/Vostok",
        member: "Antarctica_Vostok",
    },
    TimeZoneInfo {
        id: "Arctic/Longyearbyen",
        member: "Arctic_Longyearbyen",
    },
    TimeZoneInfo {
        id: "Asia/Aden",
        member: "Asia_Aden",
    },
    TimeZoneInfo {
        id: "Asia/Almaty",
        member: "Asia_Almaty",
    },
    TimeZoneInfo {
        id: "Asia/Amman",
        member: "Asia_Amman",
    },
    TimeZoneInfo {
        id: "Asia/Anadyr",
        member: "Asia_Anadyr",
    },
    TimeZoneInfo {
        id: "Asia/Aqtau",
        member: "Asia_Aqtau",
    },
    TimeZoneInfo {
        id: "Asia/Aqtobe",
        member: "Asia_Aqtobe",
    },
    TimeZoneInfo {
        id: "Asia/Ashgabat",
        member: "Asia_Ashgabat",
    },
    TimeZoneInfo {
        id: "Asia/Ashkhabad",
        member: "Asia_Ashkhabad",
    },
    TimeZoneInfo {
        id: "Asia/Atyrau",
        member: "Asia_Atyrau",
    },
    TimeZoneInfo {
        id: "Asia/Baghdad",
        member: "Asia_Baghdad",
    },
    TimeZoneInfo {
        id: "Asia/Bahrain",
        member: "Asia_Bahrain",
    },
    TimeZoneInfo {
        id: "Asia/Baku",
        member: "Asia_Baku",
    },
    TimeZoneInfo {
        id: "Asia/Bangkok",
        member: "Asia_Bangkok",
    },
    TimeZoneInfo {
        id: "Asia/Barnaul",
        member: "Asia_Barnaul",
    },
    TimeZoneInfo {
        id: "Asia/Beirut",
        member: "Asia_Beirut",
    },
    TimeZoneInfo {
        id: "Asia/Bishkek",
        member: "Asia_Bishkek",
    },
    TimeZoneInfo {
        id: "Asia/Brunei",
        member: "Asia_Brunei",
    },
    TimeZoneInfo {
        id: "Asia/Calcutta",
        member: "Asia_Calcutta",
    },
    TimeZoneInfo {
        id: "Asia/Chita",
        member: "Asia_Chita",
    },
    TimeZoneInfo {
        id: "Asia/Choibalsan",
        member: "Asia_Choibalsan",
    },
    TimeZoneInfo {
        id: "Asia/Chongqing",
        member: "Asia_Chongqing",
    },
    TimeZoneInfo {
        id: "Asia/Chungking",
        member: "Asia_Chungking",
    },
    TimeZoneInfo {
        id: "Asia/Colombo",
        member: "Asia_Colombo",
    },
    TimeZoneInfo {
        id: "Asia/Dacca",
        member: "Asia_Dacca",
    },
    TimeZoneInfo {
        id: "Asia/Damascus",
        member: "Asia_Damascus",
    },
    TimeZoneInfo {
        id: "Asia/Dhaka",
        member: "Asia_Dhaka",
    },
    TimeZoneInfo {
        id: "Asia/Dili",
        member: "Asia_Dili",
    },
    TimeZoneInfo {
        id: "Asia/Dubai",
        member: "Asia_Dubai",
    },
    TimeZoneInfo {
        id: "Asia/Dushanbe",
        member: "Asia_Dushanbe",
    },
    TimeZoneInfo {
        id: "Asia/Famagusta",
        member: "Asia_Famagusta",
    },
    TimeZoneInfo {
        id: "Asia/Gaza",
        member: "Asia_Gaza",
    },
    TimeZoneInfo {
        id: "Asia/Harbin",
        member: "Asia_Harbin",
    },
    TimeZoneInfo {
        id: "Asia/Hebron",
        member: "Asia_Hebron",
    },
    TimeZoneInfo {
        id: "Asia/Ho_Chi_Minh",
        member: "Asia_Ho_Chi_Minh",
    },
    TimeZoneInfo {
        id: "Asia/Hong_Kong",
        member: "Asia_Hong_Kong",
    },
    TimeZoneInfo {
        id: "Asia/Hovd",
        member: "Asia_Hovd",
    },
    TimeZoneInfo {
        id: "Asia/Irkutsk",
        member: "Asia_Irkutsk",
    },
    TimeZoneInfo {
        id: "Asia/Istanbul",
        member: "Asia_Istanbul",
    },
    TimeZoneInfo {
        id: "Asia/Jakarta",
        member: "Asia_Jakarta",
    },
    TimeZoneInfo {
        id: "Asia/Jayapura",
        member: "Asia_Jayapura",
    },
    TimeZoneInfo {
        id: "Asia/Jerusalem",
        member: "Asia_Jerusalem",
    },
    TimeZoneInfo {
        id: "Asia/Kabul",
        member: "Asia_Kabul",
    },
    TimeZoneInfo {
        id: "Asia/Kamchatka",
        member: "Asia_Kamchatka",
    },
    TimeZoneInfo {
        id: "Asia/Karachi",
        member: "Asia_Karachi",
    },
    TimeZoneInfo {
        id: "Asia/Kashgar",
        member: "Asia_Kashgar",
    },
    TimeZoneInfo {
        id: "Asia/Kathmandu",
        member: "Asia_Kathmandu",
    },
    TimeZoneInfo {
        id: "Asia/Katmandu",
        member: "Asia_Katmandu",
    },
    TimeZoneInfo {
        id: "Asia/Khandyga",
        member: "Asia_Khandyga",
    },
    TimeZoneInfo {
        id: "Asia/Kolkata",
        member: "Asia_Kolkata",
    },
    TimeZoneInfo {
        id: "Asia/Krasnoyarsk",
        member: "Asia_Krasnoyarsk",
    },
    TimeZoneInfo {
        id: "Asia/Kuala_Lumpur",
        member: "Asia_Kuala_Lumpur",
    },
    TimeZoneInfo {
        id: "Asia/Kuching",
        member: "Asia_Kuching",
    },
    TimeZoneInfo {
        id: "Asia/Kuwait",
        member: "Asia_Kuwait",
    },
    TimeZoneInfo {
        id: "Asia/Macao",
        member: "Asia_Macao",
    },
    TimeZoneInfo {
        id: "Asia/Macau",
        member: "Asia_Macau",
    },
    TimeZoneInfo {
        id: "Asia/Magadan",
        member: "Asia_Magadan",
    },
    TimeZoneInfo {
        id: "Asia/Makassar",
        member: "Asia_Makassar",
    },
    TimeZoneInfo {
        id: "Asia/Manila",
        member: "Asia_Manila",
    },
    TimeZoneInfo {
        id: "Asia/Muscat",
        member: "Asia_Muscat",
    },
    TimeZoneInfo {
        id: "Asia/Nicosia",
        member: "Asia_Nicosia",
    },
    TimeZoneInfo {
        id: "Asia/Novokuznetsk",
        member: "Asia_Novokuznetsk",
    },
    TimeZoneInfo {
        id: "Asia/Novosibirsk",
        member: "Asia_Novosibirsk",
    },
    TimeZoneInfo {
        id: "Asia/Omsk",
        member: "Asia_Omsk",
    },
    TimeZoneInfo {
        id: "Asia/Oral",
        member: "Asia_Oral",
    },
    TimeZoneInfo {
        id: "Asia/Phnom_Penh",
        member: "Asia_Phnom_Penh",
    },
    TimeZoneInfo {
        id: "Asia/Pontianak",
        member: "Asia_Pontianak",
    },
    TimeZoneInfo {
        id: "Asia/Pyongyang",
        member: "Asia_Pyongyang",
    },
    TimeZoneInfo {
        id: "Asia/Qatar",
        member: "Asia_Qatar",
    },
    TimeZoneInfo {
        id: "Asia/Qostanay",
        member: "Asia_Qostanay",
    },
    TimeZoneInfo {
        id: "Asia/Qyzylorda",
        member: "Asia_Qyzylorda",
    },
    TimeZoneInfo {
        id: "Asia/Rangoon",
        member: "Asia_Rangoon",
    },
    TimeZoneInfo {
        id: "Asia/Riyadh",
        member: "Asia_Riyadh",
    },
    TimeZoneInfo {
        id: "Asia/Saigon",
        member: "Asia_Saigon",
    },
    TimeZoneInfo {
        id: "Asia/Sakhalin",
        member: "Asia_Sakhalin",
    },
    TimeZoneInfo {
        id: "Asia/Samarkand",
        member: "Asia_Samarkand",
    },
    TimeZoneInfo {
        id: "Asia/Seoul",
        member: "Asia_Seoul",
    },
    TimeZoneInfo {
        id: "Asia/Shanghai",
        member: "Asia_Shanghai",
    },
    TimeZoneInfo {
        id: "Asia/Singapore",
        member: "Asia_Singapore",
    },
    TimeZoneInfo {
        id: "Asia/Srednekolymsk",
        member: "Asia_Srednekolymsk",
    },
    TimeZoneInfo {
        id: "Asia/Taipei",
        member: "Asia_Taipei",
    },
    TimeZoneInfo {
        id: "Asia/Tashkent",
        member: "Asia_Tashkent",
    },
    TimeZoneInfo {
        id: "Asia/Tbilisi",
        member: "Asia_Tbilisi",
    },
    TimeZoneInfo {
        id: "Asia/Tehran",
        member: "Asia_Tehran",
    },
    TimeZoneInfo {
        id: "Asia/Tel_Aviv",
        member: "Asia_Tel_Aviv",
    },
    TimeZoneInfo {
        id: "Asia/Thimbu",
        member: "Asia_Thimbu",
    },
    TimeZoneInfo {
        id: "Asia/Thimphu",
        member: "Asia_Thimphu",
    },
    TimeZoneInfo {
        id: "Asia/Tokyo",
        member: "Asia_Tokyo",
    },
    TimeZoneInfo {
        id: "Asia/Tomsk",
        member: "Asia_Tomsk",
    },
    TimeZoneInfo {
        id: "Asia/Ujung_Pandang",
        member: "Asia_Ujung_Pandang",
    },
    TimeZoneInfo {
        id: "Asia/Ulaanbaatar",
        member: "Asia_Ulaanbaatar",
    },
    TimeZoneInfo {
        id: "Asia/Ulan_Bator",
        member: "Asia_Ulan_Bator",
    },
    TimeZoneInfo {
        id: "Asia/Urumqi",
        member: "Asia_Urumqi",
    },
    TimeZoneInfo {
        id: "Asia/Ust-Nera",
        member: "Asia_Ust_Minus_Nera",
    },
    TimeZoneInfo {
        id: "Asia/Vientiane",
        member: "Asia_Vientiane",
    },
    TimeZoneInfo {
        id: "Asia/Vladivostok",
        member: "Asia_Vladivostok",
    },
    TimeZoneInfo {
        id: "Asia/Yakutsk",
        member: "Asia_Yakutsk",
    },
    TimeZoneInfo {
        id: "Asia/Yangon",
        member: "Asia_Yangon",
    },
    TimeZoneInfo {
        id: "Asia/Yekaterinburg",
        member: "Asia_Yekaterinburg",
    },
    TimeZoneInfo {
        id: "Asia/Yerevan",
        member: "Asia_Yerevan",
    },
    TimeZoneInfo {
        id: "Atlantic/Azores",
        member: "Atlantic_Azores",
    },
    TimeZoneInfo {
        id: "Atlantic/Bermuda",
        member: "Atlantic_Bermuda",
    },
    TimeZoneInfo {
        id: "Atlantic/Canary",
        member: "Atlantic_Canary",
    },
    TimeZoneInfo {
        id: "Atlantic/Cape_Verde",
        member: "Atlantic_Cape_Verde",
    },
    TimeZoneInfo {
        id: "Atlantic/Faeroe",
        member: "Atlantic_Faeroe",
    },
    TimeZoneInfo {
        id: "Atlantic/Faroe",
        member: "Atlantic_Faroe",
    },
    TimeZoneInfo {
        id: "Atlantic/Jan_Mayen",
        member: "Atlantic_Jan_Mayen",
    },
    TimeZoneInfo {
        id: "Atlantic/Madeira",
        member: "Atlantic_Madeira",
    },
    TimeZoneInfo {
        id: "Atlantic/Reykjavik",
        member: "Atlantic_Reykjavik",
    },
    TimeZoneInfo {
        id: "Atlantic/South_Georgia",
        member: "Atlantic_South_Georgia",
    },
    TimeZoneInfo {
        id: "Atlantic/St_Helena",
        member: "Atlantic_St_Helena",
    },
    TimeZoneInfo {
        id: "Atlantic/Stanley",
        member: "Atlantic_Stanley",
    },
    TimeZoneInfo {
        id: "Australia/ACT",
        member: "Australia_ACT",
    },
    TimeZoneInfo {
        id: "Australia/Adelaide",
        member: "Australia_Adelaide",
    },
    TimeZoneInfo {
        id: "Australia/Brisbane",
        member: "Australia_Brisbane",
    },
    TimeZoneInfo {
        id: "Australia/Broken_Hill",
        member: "Australia_Broken_Hill",
    },
    TimeZoneInfo {
        id: "Australia/Canberra",
        member: "Australia_Canberra",
    },
    TimeZoneInfo {
        id: "Australia/Currie",
        member: "Australia_Currie",
    },
    TimeZoneInfo {
        id: "Australia/Darwin",
        member: "Australia_Darwin",
    },
    TimeZoneInfo {
        id: "Australia/Eucla",
        member: "Australia_Eucla",
    },
    TimeZoneInfo {
        id: "Australia/Hobart",
        member: "Australia_Hobart",
    },
    TimeZoneInfo {
        id: "Australia/LHI",
        member: "Australia_LHI",
    },
    TimeZoneInfo {
        id: "Australia/Lindeman",
        member: "Australia_Lindeman",
    },
    TimeZoneInfo {
        id: "Australia/Lord_Howe",
        member: "Australia_Lord_Howe",
    },
    TimeZoneInfo {
        id: "Australia/Melbourne",
        member: "Australia_Melbourne",
    },
    TimeZoneInfo {
        id: "Australia/NSW",
        member: "Australia_NSW",
    },
    TimeZoneInfo {
        id: "Australia/North",
        member: "Australia_North",
    },
    TimeZoneInfo {
        id: "Australia/Perth",
        member: "Australia_Perth",
    },
    TimeZoneInfo {
        id: "Australia/Queensland",
        member: "Australia_Queensland",
    },
    TimeZoneInfo {
        id: "Australia/South",
        member: "Australia_South",
    },
    TimeZoneInfo {
        id: "Australia/Sydney",
        member: "Australia_Sydney",
    },
    TimeZoneInfo {
        id: "Australia/Tasmania",
        member: "Australia_Tasmania",
    },
    TimeZoneInfo {
        id: "Australia/Victoria",
        member: "Australia_Victoria",
    },
    TimeZoneInfo {
        id: "Australia/West",
        member: "Australia_West",
    },
    TimeZoneInfo {
        id: "Australia/Yancowinna",
        member: "Australia_Yancowinna",
    },
    TimeZoneInfo {
        id: "Brazil/Acre",
        member: "Brazil_Acre",
    },
    TimeZoneInfo {
        id: "Brazil/DeNoronha",
        member: "Brazil_DeNoronha",
    },
    TimeZoneInfo {
        id: "Brazil/East",
        member: "Brazil_East",
    },
    TimeZoneInfo {
        id: "Brazil/West",
        member: "Brazil_West",
    },
    TimeZoneInfo {
        id: "CET",
        member: "CET",
    },
    TimeZoneInfo {
        id: "CST6CDT",
        member: "CST6CDT",
    },
    TimeZoneInfo {
        id: "Canada/Atlantic",
        member: "Canada_Atlantic",
    },
    TimeZoneInfo {
        id: "Canada/Central",
        member: "Canada_Central",
    },
    TimeZoneInfo {
        id: "Canada/Eastern",
        member: "Canada_Eastern",
    },
    TimeZoneInfo {
        id: "Canada/Mountain",
        member: "Canada_Mountain",
    },
    TimeZoneInfo {
        id: "Canada/Newfoundland",
        member: "Canada_Newfoundland",
    },
    TimeZoneInfo {
        id: "Canada/Pacific",
        member: "Canada_Pacific",
    },
    TimeZoneInfo {
        id: "Canada/Saskatchewan",
        member: "Canada_Saskatchewan",
    },
    TimeZoneInfo {
        id: "Canada/Yukon",
        member: "Canada_Yukon",
    },
    TimeZoneInfo {
        id: "Chile/Continental",
        member: "Chile_Continental",
    },
    TimeZoneInfo {
        id: "Chile/EasterIsland",
        member: "Chile_EasterIsland",
    },
    TimeZoneInfo {
        id: "Cuba",
        member: "Cuba",
    },
    TimeZoneInfo {
        id: "EET",
        member: "EET",
    },
    TimeZoneInfo {
        id: "EST",
        member: "EST",
    },
    TimeZoneInfo {
        id: "EST5EDT",
        member: "EST5EDT",
    },
    TimeZoneInfo {
        id: "Egypt",
        member: "Egypt",
    },
    TimeZoneInfo {
        id: "Eire",
        member: "Eire",
    },
    TimeZoneInfo {
        id: "Etc/GMT",
        member: "Etc_GMT",
    },
    TimeZoneInfo {
        id: "Etc/GMT+0",
        member: "Etc_GMT_Plus_0",
    },
    TimeZoneInfo {
        id: "Etc/GMT+1",
        member: "Etc_GMT_Plus_1",
    },
    TimeZoneInfo {
        id: "Etc/GMT+10",
        member: "Etc_GMT_Plus_10",
    },
    TimeZoneInfo {
        id: "Etc/GMT+11",
        member: "Etc_GMT_Plus_11",
    },
    TimeZoneInfo {
        id: "Etc/GMT+12",
        member: "Etc_GMT_Plus_12",
    },
    TimeZoneInfo {
        id: "Etc/GMT+2",
        member: "Etc_GMT_Plus_2",
    },
    TimeZoneInfo {
        id: "Etc/GMT+3",
        member: "Etc_GMT_Plus_3",
    },
    TimeZoneInfo {
        id: "Etc/GMT+4",
        member: "Etc_GMT_Plus_4",
    },
    TimeZoneInfo {
        id: "Etc/GMT+5",
        member: "Etc_GMT_Plus_5",
    },
    TimeZoneInfo {
        id: "Etc/GMT+6",
        member: "Etc_GMT_Plus_6",
    },
    TimeZoneInfo {
        id: "Etc/GMT+7",
        member: "Etc_GMT_Plus_7",
    },
    TimeZoneInfo {
        id: "Etc/GMT+8",
        member: "Etc_GMT_Plus_8",
    },
    TimeZoneInfo {
        id: "Etc/GMT+9",
        member: "Etc_GMT_Plus_9",
    },
    TimeZoneInfo {
        id: "Etc/GMT-0",
        member: "Etc_GMT_Minus_0",
    },
    TimeZoneInfo {
        id: "Etc/GMT-1",
        member: "Etc_GMT_Minus_1",
    },
    TimeZoneInfo {
        id: "Etc/GMT-10",
        member: "Etc_GMT_Minus_10",
    },
    TimeZoneInfo {
        id: "Etc/GMT-11",
        member: "Etc_GMT_Minus_11",
    },
    TimeZoneInfo {
        id: "Etc/GMT-12",
        member: "Etc_GMT_Minus_12",
    },
    TimeZoneInfo {
        id: "Etc/GMT-13",
        member: "Etc_GMT_Minus_13",
    },
    TimeZoneInfo {
        id: "Etc/GMT-14",
        member: "Etc_GMT_Minus_14",
    },
    TimeZoneInfo {
        id: "Etc/GMT-2",
        member: "Etc_GMT_Minus_2",
    },
    TimeZoneInfo {
        id: "Etc/GMT-3",
        member: "Etc_GMT_Minus_3",
    },
    TimeZoneInfo {
        id: "Etc/GMT-4",
        member: "Etc_GMT_Minus_4",
    },
    TimeZoneInfo {
        id: "Etc/GMT-5",
        member: "Etc_GMT_Minus_5",
    },
    TimeZoneInfo {
        id: "Etc/GMT-6",
        member: "Etc_GMT_Minus_6",
    },
    TimeZoneInfo {
        id: "Etc/GMT-7",
        member: "Etc_GMT_Minus_7",
    },
    TimeZoneInfo {
        id: "Etc/GMT-8",
        member: "Etc_GMT_Minus_8",
    },
    TimeZoneInfo {
        id: "Etc/GMT-9",
        member: "Etc_GMT_Minus_9",
    },
    TimeZoneInfo {
        id: "Etc/GMT0",
        member: "Etc_GMT0",
    },
    TimeZoneInfo {
        id: "Etc/Greenwich",
        member: "Etc_Greenwich",
    },
    TimeZoneInfo {
        id: "Etc/UCT",
        member: "Etc_UCT",
    },
    TimeZoneInfo {
        id: "Etc/UTC",
        member: "Etc_UTC",
    },
    TimeZoneInfo {
        id: "Etc/Universal",
        member: "Etc_Universal",
    },
    TimeZoneInfo {
        id: "Etc/Zulu",
        member: "Etc_Zulu",
    },
    TimeZoneInfo {
        id: "Europe/Amsterdam",
        member: "Europe_Amsterdam",
    },
    TimeZoneInfo {
        id: "Europe/Andorra",
        member: "Europe_Andorra",
    },
    TimeZoneInfo {
        id: "Europe/Astrakhan",
        member: "Europe_Astrakhan",
    },
    TimeZoneInfo {
        id: "Europe/Athens",
        member: "Europe_Athens",
    },
    TimeZoneInfo {
        id: "Europe/Belfast",
        member: "Europe_Belfast",
    },
    TimeZoneInfo {
        id: "Europe/Belgrade",
        member: "Europe_Belgrade",
    },
    TimeZoneInfo {
        id: "Europe/Berlin",
        member: "Europe_Berlin",
    },
    TimeZoneInfo {
        id: "Europe/Bratislava",
        member: "Europe_Bratislava",
    },
    TimeZoneInfo {
        id: "Europe/Brussels",
        member: "Europe_Brussels",
    },
    TimeZoneInfo {
        id: "Europe/Bucharest",
        member: "Europe_Bucharest",
    },
    TimeZoneInfo {
        id: "Europe/Budapest",
        member: "Europe_Budapest",
    },
    TimeZoneInfo {
        id: "Europe/Busingen",
        member: "Europe_Busingen",
    },
    TimeZoneInfo {
        id: "Europe/Chisinau",
        member: "Europe_Chisinau",
    },
    TimeZoneInfo {
        id: "Europe/Copenhagen",
        member: "Europe_Copenhagen",
    },
    TimeZoneInfo {
        id: "Europe/Dublin",
        member: "Europe_Dublin",
    },
    TimeZoneInfo {
        id: "Europe/Gibraltar",
        member: "Europe_Gibraltar",
    },
    TimeZoneInfo {
        id: "Europe/Guernsey",
        member: "Europe_Guernsey",
    },
    TimeZoneInfo {
        id: "Europe/Helsinki",
        member: "Europe_Helsinki",
    },
    TimeZoneInfo {
        id: "Europe/Isle_of_Man",
        member: "Europe_Isle_of_Man",
    },
    TimeZoneInfo {
        id: "Europe/Istanbul",
        member: "Europe_Istanbul",
    },
    TimeZoneInfo {
        id: "Europe/Jersey",
        member: "Europe_Jersey",
    },
    TimeZoneInfo {
        id: "Europe/Kaliningrad",
        member: "Europe_Kaliningrad",
    },
    TimeZoneInfo {
        id: "Europe/Kiev",
        member: "Europe_Kiev",
    },
    TimeZoneInfo {
        id: "Europe/Kirov",
        member: "Europe_Kirov",
    },
    TimeZoneInfo {
        id: "Europe/Kyiv",
        member: "Europe_Kyiv",
    },
    TimeZoneInfo {
        id: "Europe/Lisbon",
        member: "Europe_Lisbon",
    },
    TimeZoneInfo {
        id: "Europe/Ljubljana",
        member: "Europe_Ljubljana",
    },
    TimeZoneInfo {
        id: "Europe/London",
        member: "Europe_London",
    },
    TimeZoneInfo {
        id: "Europe/Luxembourg",
        member: "Europe_Luxembourg",
    },
    TimeZoneInfo {
        id: "Europe/Madrid",
        member: "Europe_Madrid",
    },
    TimeZoneInfo {
        id: "Europe/Malta",
        member: "Europe_Malta",
    },
    TimeZoneInfo {
        id: "Europe/Mariehamn",
        member: "Europe_Mariehamn",
    },
    TimeZoneInfo {
        id: "Europe/Minsk",
        member: "Europe_Minsk",
    },
    TimeZoneInfo {
        id: "Europe/Monaco",
        member: "Europe_Monaco",
    },
    TimeZoneInfo {
        id: "Europe/Moscow",
        member: "Europe_Moscow",
    },
    TimeZoneInfo {
        id: "Europe/Nicosia",
        member: "Europe_Nicosia",
    },
    TimeZoneInfo {
        id: "Europe/Oslo",
        member: "Europe_Oslo",
    },
    TimeZoneInfo {
        id: "Europe/Paris",
        member: "Europe_Paris",
    },
    TimeZoneInfo {
        id: "Europe/Podgorica",
        member: "Europe_Podgorica",
    },
    TimeZoneInfo {
        id: "Europe/Prague",
        member: "Europe_Prague",
    },
    TimeZoneInfo {
        id: "Europe/Riga",
        member: "Europe_Riga",
    },
    TimeZoneInfo {
        id: "Europe/Rome",
        member: "Europe_Rome",
    },
    TimeZoneInfo {
        id: "Europe/Samara",
        member: "Europe_Samara",
    },
    TimeZoneInfo {
        id: "Europe/San_Marino",
        member: "Europe_San_Marino",
    },
    TimeZoneInfo {
        id: "Europe/Sarajevo",
        member: "Europe_Sarajevo",
    },
    TimeZoneInfo {
        id: "Europe/Saratov",
        member: "Europe_Saratov",
    },
    TimeZoneInfo {
        id: "Europe/Simferopol",
        member: "Europe_Simferopol",
    },
    TimeZoneInfo {
        id: "Europe/Skopje",
        member: "Europe_Skopje",
    },
    TimeZoneInfo {
        id: "Europe/Sofia",
        member: "Europe_Sofia",
    },
    TimeZoneInfo {
        id: "Europe/Stockholm",
        member: "Europe_Stockholm",
    },
    TimeZoneInfo {
        id: "Europe/Tallinn",
        member: "Europe_Tallinn",
    },
    TimeZoneInfo {
        id: "Europe/Tirane",
        member: "Europe_Tirane",
    },
    TimeZoneInfo {
        id: "Europe/Tiraspol",
        member: "Europe_Tiraspol",
    },
    TimeZoneInfo {
        id: "Europe/Ulyanovsk",
        member: "Europe_Ulyanovsk",
    },
    TimeZoneInfo {
        id: "Europe/Uzhgorod",
        member: "Europe_Uzhgorod",
    },
    TimeZoneInfo {
        id: "Europe/Vaduz",
        member: "Europe_Vaduz",
    },
    TimeZoneInfo {
        id: "Europe/Vatican",
        member: "Europe_Vatican",
    },
    TimeZoneInfo {
        id: "Europe/Vienna",
        member: "Europe_Vienna",
    },
    TimeZoneInfo {
        id: "Europe/Vilnius",
        member: "Europe_Vilnius",
    },
    TimeZoneInfo {
        id: "Europe/Volgograd",
        member: "Europe_Volgograd",
    },
    TimeZoneInfo {
        id: "Europe/Warsaw",
        member: "Europe_Warsaw",
    },
    TimeZoneInfo {
        id: "Europe/Zagreb",
        member: "Europe_Zagreb",
    },
    TimeZoneInfo {
        id: "Europe/Zaporozhye",
        member: "Europe_Zaporozhye",
    },
    TimeZoneInfo {
        id: "Europe/Zurich",
        member: "Europe_Zurich",
    },
    TimeZoneInfo {
        id: "Factory",
        member: "Factory",
    },
    TimeZoneInfo {
        id: "GB",
        member: "GB",
    },
    TimeZoneInfo {
        id: "GB-Eire",
        member: "GB_Minus_Eire",
    },
    TimeZoneInfo {
        id: "GMT",
        member: "GMT",
    },
    TimeZoneInfo {
        id: "GMT+0",
        member: "GMT_Plus_0",
    },
    TimeZoneInfo {
        id: "GMT-0",
        member: "GMT_Minus_0",
    },
    TimeZoneInfo {
        id: "GMT0",
        member: "GMT0",
    },
    TimeZoneInfo {
        id: "Greenwich",
        member: "Greenwich",
    },
    TimeZoneInfo {
        id: "HST",
        member: "HST",
    },
    TimeZoneInfo {
        id: "Hongkong",
        member: "Hongkong",
    },
    TimeZoneInfo {
        id: "Iceland",
        member: "Iceland",
    },
    TimeZoneInfo {
        id: "Indian/Antananarivo",
        member: "Indian_Antananarivo",
    },
    TimeZoneInfo {
        id: "Indian/Chagos",
        member: "Indian_Chagos",
    },
    TimeZoneInfo {
        id: "Indian/Christmas",
        member: "Indian_Christmas",
    },
    TimeZoneInfo {
        id: "Indian/Cocos",
        member: "Indian_Cocos",
    },
    TimeZoneInfo {
        id: "Indian/Comoro",
        member: "Indian_Comoro",
    },
    TimeZoneInfo {
        id: "Indian/Kerguelen",
        member: "Indian_Kerguelen",
    },
    TimeZoneInfo {
        id: "Indian/Mahe",
        member: "Indian_Mahe",
    },
    TimeZoneInfo {
        id: "Indian/Maldives",
        member: "Indian_Maldives",
    },
    TimeZoneInfo {
        id: "Indian/Mauritius",
        member: "Indian_Mauritius",
    },
    TimeZoneInfo {
        id: "Indian/Mayotte",
        member: "Indian_Mayotte",
    },
    TimeZoneInfo {
        id: "Indian/Reunion",
        member: "Indian_Reunion",
    },
    TimeZoneInfo {
        id: "Iran",
        member: "Iran",
    },
    TimeZoneInfo {
        id: "Israel",
        member: "Israel",
    },
    TimeZoneInfo {
        id: "Jamaica",
        member: "Jamaica",
    },
    TimeZoneInfo {
        id: "Japan",
        member: "Japan",
    },
    TimeZoneInfo {
        id: "Kwajalein",
        member: "Kwajalein",
    },
    TimeZoneInfo {
        id: "Libya",
        member: "Libya",
    },
    TimeZoneInfo {
        id: "MET",
        member: "MET",
    },
    TimeZoneInfo {
        id: "MST",
        member: "MST",
    },
    TimeZoneInfo {
        id: "MST7MDT",
        member: "MST7MDT",
    },
    TimeZoneInfo {
        id: "Mexico/BajaNorte",
        member: "Mexico_BajaNorte",
    },
    TimeZoneInfo {
        id: "Mexico/BajaSur",
        member: "Mexico_BajaSur",
    },
    TimeZoneInfo {
        id: "Mexico/General",
        member: "Mexico_General",
    },
    TimeZoneInfo {
        id: "NZ",
        member: "NZ",
    },
    TimeZoneInfo {
        id: "NZ-CHAT",
        member: "NZ_Minus_CHAT",
    },
    TimeZoneInfo {
        id: "Navajo",
        member: "Navajo",
    },
    TimeZoneInfo {
        id: "PRC",
        member: "PRC",
    },
    TimeZoneInfo {
        id: "PST8PDT",
        member: "PST8PDT",
    },
    TimeZoneInfo {
        id: "Pacific/Apia",
        member: "Pacific_Apia",
    },
    TimeZoneInfo {
        id: "Pacific/Auckland",
        member: "Pacific_Auckland",
    },
    TimeZoneInfo {
        id: "Pacific/Bougainville",
        member: "Pacific_Bougainville",
    },
    TimeZoneInfo {
        id: "Pacific/Chatham",
        member: "Pacific_Chatham",
    },
    TimeZoneInfo {
        id: "Pacific/Chuuk",
        member: "Pacific_Chuuk",
    },
    TimeZoneInfo {
        id: "Pacific/Easter",
        member: "Pacific_Easter",
    },
    TimeZoneInfo {
        id: "Pacific/Efate",
        member: "Pacific_Efate",
    },
    TimeZoneInfo {
        id: "Pacific/Enderbury",
        member: "Pacific_Enderbury",
    },
    TimeZoneInfo {
        id: "Pacific/Fakaofo",
        member: "Pacific_Fakaofo",
    },
    TimeZoneInfo {
        id: "Pacific/Fiji",
        member: "Pacific_Fiji",
    },
    TimeZoneInfo {
        id: "Pacific/Funafuti",
        member: "Pacific_Funafuti",
    },
    TimeZoneInfo {
        id: "Pacific/Galapagos",
        member: "Pacific_Galapagos",
    },
    TimeZoneInfo {
        id: "Pacific/Gambier",
        member: "Pacific_Gambier",
    },
    TimeZoneInfo {
        id: "Pacific/Guadalcanal",
        member: "Pacific_Guadalcanal",
    },
    TimeZoneInfo {
        id: "Pacific/Guam",
        member: "Pacific_Guam",
    },
    TimeZoneInfo {
        id: "Pacific/Honolulu",
        member: "Pacific_Honolulu",
    },
    TimeZoneInfo {
        id: "Pacific/Johnston",
        member: "Pacific_Johnston",
    },
    TimeZoneInfo {
        id: "Pacific/Kanton",
        member: "Pacific_Kanton",
    },
    TimeZoneInfo {
        id: "Pacific/Kiritimati",
        member: "Pacific_Kiritimati",
    },
    TimeZoneInfo {
        id: "Pacific/Kosrae",
        member: "Pacific_Kosrae",
    },
    TimeZoneInfo {
        id: "Pacific/Kwajalein",
        member: "Pacific_Kwajalein",
    },
    TimeZoneInfo {
        id: "Pacific/Majuro",
        member: "Pacific_Majuro",
    },
    TimeZoneInfo {
        id: "Pacific/Marquesas",
        member: "Pacific_Marquesas",
    },
    TimeZoneInfo {
        id: "Pacific/Midway",
        member: "Pacific_Midway",
    },
    TimeZoneInfo {
        id: "Pacific/Nauru",
        member: "Pacific_Nauru",
    },
    TimeZoneInfo {
        id: "Pacific/Niue",
        member: "Pacific_Niue",
    },
    TimeZoneInfo {
        id: "Pacific/Norfolk",
        member: "Pacific_Norfolk",
    },
    TimeZoneInfo {
        id: "Pacific/Noumea",
        member: "Pacific_Noumea",
    },
    TimeZoneInfo {
        id: "Pacific/Pago_Pago",
        member: "Pacific_Pago_Pago",
    },
    TimeZoneInfo {
        id: "Pacific/Palau",
        member: "Pacific_Palau",
    },
    TimeZoneInfo {
        id: "Pacific/Pitcairn",
        member: "Pacific_Pitcairn",
    },
    TimeZoneInfo {
        id: "Pacific/Pohnpei",
        member: "Pacific_Pohnpei",
    },
    TimeZoneInfo {
        id: "Pacific/Ponape",
        member: "Pacific_Ponape",
    },
    TimeZoneInfo {
        id: "Pacific/Port_Moresby",
        member: "Pacific_Port_Moresby",
    },
    TimeZoneInfo {
        id: "Pacific/Rarotonga",
        member: "Pacific_Rarotonga",
    },
    TimeZoneInfo {
        id: "Pacific/Saipan",
        member: "Pacific_Saipan",
    },
    TimeZoneInfo {
        id: "Pacific/Samoa",
        member: "Pacific_Samoa",
    },
    TimeZoneInfo {
        id: "Pacific/Tahiti",
        member: "Pacific_Tahiti",
    },
    TimeZoneInfo {
        id: "Pacific/Tarawa",
        member: "Pacific_Tarawa",
    },
    TimeZoneInfo {
        id: "Pacific/Tongatapu",
        member: "Pacific_Tongatapu",
    },
    TimeZoneInfo {
        id: "Pacific/Truk",
        member: "Pacific_Truk",
    },
    TimeZoneInfo {
        id: "Pacific/Wake",
        member: "Pacific_Wake",
    },
    TimeZoneInfo {
        id: "Pacific/Wallis",
        member: "Pacific_Wallis",
    },
    TimeZoneInfo {
        id: "Pacific/Yap",
        member: "Pacific_Yap",
    },
    TimeZoneInfo {
        id: "Poland",
        member: "Poland",
    },
    TimeZoneInfo {
        id: "Portugal",
        member: "Portugal",
    },
    TimeZoneInfo {
        id: "ROC",
        member: "ROC",
    },
    TimeZoneInfo {
        id: "ROK",
        member: "ROK",
    },
    TimeZoneInfo {
        id: "Singapore",
        member: "Singapore",
    },
    TimeZoneInfo {
        id: "Turkey",
        member: "Turkey",
    },
    TimeZoneInfo {
        id: "UCT",
        member: "UCT",
    },
    TimeZoneInfo {
        id: "US/Alaska",
        member: "US_Alaska",
    },
    TimeZoneInfo {
        id: "US/Aleutian",
        member: "US_Aleutian",
    },
    TimeZoneInfo {
        id: "US/Arizona",
        member: "US_Arizona",
    },
    TimeZoneInfo {
        id: "US/Central",
        member: "US_Central",
    },
    TimeZoneInfo {
        id: "US/East-Indiana",
        member: "US_East_Minus_Indiana",
    },
    TimeZoneInfo {
        id: "US/Eastern",
        member: "US_Eastern",
    },
    TimeZoneInfo {
        id: "US/Hawaii",
        member: "US_Hawaii",
    },
    TimeZoneInfo {
        id: "US/Indiana-Starke",
        member: "US_Indiana_Minus_Starke",
    },
    TimeZoneInfo {
        id: "US/Michigan",
        member: "US_Michigan",
    },
    TimeZoneInfo {
        id: "US/Mountain",
        member: "US_Mountain",
    },
    TimeZoneInfo {
        id: "US/Pacific",
        member: "US_Pacific",
    },
    TimeZoneInfo {
        id: "US/Samoa",
        member: "US_Samoa",
    },
    TimeZoneInfo {
        id: "UTC",
        member: "UTC",
    },
    TimeZoneInfo {
        id: "Universal",
        member: "Universal",
    },
    TimeZoneInfo {
        id: "W-SU",
        member: "W_Minus_SU",
    },
    TimeZoneInfo {
        id: "WET",
        member: "WET",
    },
    TimeZoneInfo {
        id: "Zulu",
        member: "Zulu",
    },
];

#[allow(clippy::too_many_lines)]
pub fn ordinal(name: &str) -> Option<i32> {
    Some(match name {
        "Africa/Abidjan" => 0,
        "Africa/Accra" => 1,
        "Africa/Addis_Ababa" => 2,
        "Africa/Algiers" => 3,
        "Africa/Asmara" => 4,
        "Africa/Asmera" => 5,
        "Africa/Bamako" => 6,
        "Africa/Bangui" => 7,
        "Africa/Banjul" => 8,
        "Africa/Bissau" => 9,
        "Africa/Blantyre" => 10,
        "Africa/Brazzaville" => 11,
        "Africa/Bujumbura" => 12,
        "Africa/Cairo" => 13,
        "Africa/Casablanca" => 14,
        "Africa/Ceuta" => 15,
        "Africa/Conakry" => 16,
        "Africa/Dakar" => 17,
        "Africa/Dar_es_Salaam" => 18,
        "Africa/Djibouti" => 19,
        "Africa/Douala" => 20,
        "Africa/El_Aaiun" => 21,
        "Africa/Freetown" => 22,
        "Africa/Gaborone" => 23,
        "Africa/Harare" => 24,
        "Africa/Johannesburg" => 25,
        "Africa/Juba" => 26,
        "Africa/Kampala" => 27,
        "Africa/Khartoum" => 28,
        "Africa/Kigali" => 29,
        "Africa/Kinshasa" => 30,
        "Africa/Lagos" => 31,
        "Africa/Libreville" => 32,
        "Africa/Lome" => 33,
        "Africa/Luanda" => 34,
        "Africa/Lubumbashi" => 35,
        "Africa/Lusaka" => 36,
        "Africa/Malabo" => 37,
        "Africa/Maputo" => 38,
        "Africa/Maseru" => 39,
        "Africa/Mbabane" => 40,
        "Africa/Mogadishu" => 41,
        "Africa/Monrovia" => 42,
        "Africa/Nairobi" => 43,
        "Africa/Ndjamena" => 44,
        "Africa/Niamey" => 45,
        "Africa/Nouakchott" => 46,
        "Africa/Ouagadougou" => 47,
        "Africa/Porto-Novo" => 48,
        "Africa/Sao_Tome" => 49,
        "Africa/Timbuktu" => 50,
        "Africa/Tripoli" => 51,
        "Africa/Tunis" => 52,
        "Africa/Windhoek" => 53,
        "America/Adak" => 54,
        "America/Anchorage" => 55,
        "America/Anguilla" => 56,
        "America/Antigua" => 57,
        "America/Araguaina" => 58,
        "America/Argentina/Buenos_Aires" => 59,
        "America/Argentina/Catamarca" => 60,
        "America/Argentina/ComodRivadavia" => 61,
        "America/Argentina/Cordoba" => 62,
        "America/Argentina/Jujuy" => 63,
        "America/Argentina/La_Rioja" => 64,
        "America/Argentina/Mendoza" => 65,
        "America/Argentina/Rio_Gallegos" => 66,
        "America/Argentina/Salta" => 67,
        "America/Argentina/San_Juan" => 68,
        "America/Argentina/San_Luis" => 69,
        "America/Argentina/Tucuman" => 70,
        "America/Argentina/Ushuaia" => 71,
        "America/Aruba" => 72,
        "America/Asuncion" => 73,
        "America/Atikokan" => 74,
        "America/Atka" => 75,
        "America/Bahia" => 76,
        "America/Bahia_Banderas" => 77,
        "America/Barbados" => 78,
        "America/Belem" => 79,
        "America/Belize" => 80,
        "America/Blanc-Sablon" => 81,
        "America/Boa_Vista" => 82,
        "America/Bogota" => 83,
        "America/Boise" => 84,
        "America/Buenos_Aires" => 85,
        "America/Cambridge_Bay" => 86,
        "America/Campo_Grande" => 87,
        "America/Cancun" => 88,
        "America/Caracas" => 89,
        "America/Catamarca" => 90,
        "America/Cayenne" => 91,
        "America/Cayman" => 92,
        "America/Chicago" => 93,
        "America/Chihuahua" => 94,
        "America/Ciudad_Juarez" => 95,
        "America/Coral_Harbour" => 96,
        "America/Cordoba" => 97,
        "America/Costa_Rica" => 98,
        "America/Coyhaique" => 99,
        "America/Creston" => 100,
        "America/Cuiaba" => 101,
        "America/Curacao" => 102,
        "America/Danmarkshavn" => 103,
        "America/Dawson" => 104,
        "America/Dawson_Creek" => 105,
        "America/Denver" => 106,
        "America/Detroit" => 107,
        "America/Dominica" => 108,
        "America/Edmonton" => 109,
        "America/Eirunepe" => 110,
        "America/El_Salvador" => 111,
        "America/Ensenada" => 112,
        "America/Fort_Nelson" => 113,
        "America/Fort_Wayne" => 114,
        "America/Fortaleza" => 115,
        "America/Glace_Bay" => 116,
        "America/Godthab" => 117,
        "America/Goose_Bay" => 118,
        "America/Grand_Turk" => 119,
        "America/Grenada" => 120,
        "America/Guadeloupe" => 121,
        "America/Guatemala" => 122,
        "America/Guayaquil" => 123,
        "America/Guyana" => 124,
        "America/Halifax" => 125,
        "America/Havana" => 126,
        "America/Hermosillo" => 127,
        "America/Indiana/Indianapolis" => 128,
        "America/Indiana/Knox" => 129,
        "America/Indiana/Marengo" => 130,
        "America/Indiana/Petersburg" => 131,
        "America/Indiana/Tell_City" => 132,
        "America/Indiana/Vevay" => 133,
        "America/Indiana/Vincennes" => 134,
        "America/Indiana/Winamac" => 135,
        "America/Indianapolis" => 136,
        "America/Inuvik" => 137,
        "America/Iqaluit" => 138,
        "America/Jamaica" => 139,
        "America/Jujuy" => 140,
        "America/Juneau" => 141,
        "America/Kentucky/Louisville" => 142,
        "America/Kentucky/Monticello" => 143,
        "America/Knox_IN" => 144,
        "America/Kralendijk" => 145,
        "America/La_Paz" => 146,
        "America/Lima" => 147,
        "America/Los_Angeles" => 148,
        "America/Louisville" => 149,
        "America/Lower_Princes" => 150,
        "America/Maceio" => 151,
        "America/Managua" => 152,
        "America/Manaus" => 153,
        "America/Marigot" => 154,
        "America/Martinique" => 155,
        "America/Matamoros" => 156,
        "America/Mazatlan" => 157,
        "America/Mendoza" => 158,
        "America/Menominee" => 159,
        "America/Merida" => 160,
        "America/Metlakatla" => 161,
        "America/Mexico_City" => 162,
        "America/Miquelon" => 163,
        "America/Moncton" => 164,
        "America/Monterrey" => 165,
        "America/Montevideo" => 166,
        "America/Montreal" => 167,
        "America/Montserrat" => 168,
        "America/Nassau" => 169,
        "America/New_York" => 170,
        "America/Nipigon" => 171,
        "America/Nome" => 172,
        "America/Noronha" => 173,
        "America/North_Dakota/Beulah" => 174,
        "America/North_Dakota/Center" => 175,
        "America/North_Dakota/New_Salem" => 176,
        "America/Nuuk" => 177,
        "America/Ojinaga" => 178,
        "America/Panama" => 179,
        "America/Pangnirtung" => 180,
        "America/Paramaribo" => 181,
        "America/Phoenix" => 182,
        "America/Port-au-Prince" => 183,
        "America/Port_of_Spain" => 184,
        "America/Porto_Acre" => 185,
        "America/Porto_Velho" => 186,
        "America/Puerto_Rico" => 187,
        "America/Punta_Arenas" => 188,
        "America/Rainy_River" => 189,
        "America/Rankin_Inlet" => 190,
        "America/Recife" => 191,
        "America/Regina" => 192,
        "America/Resolute" => 193,
        "America/Rio_Branco" => 194,
        "America/Rosario" => 195,
        "America/Santa_Isabel" => 196,
        "America/Santarem" => 197,
        "America/Santiago" => 198,
        "America/Santo_Domingo" => 199,
        "America/Sao_Paulo" => 200,
        "America/Scoresbysund" => 201,
        "America/Shiprock" => 202,
        "America/Sitka" => 203,
        "America/St_Barthelemy" => 204,
        "America/St_Johns" => 205,
        "America/St_Kitts" => 206,
        "America/St_Lucia" => 207,
        "America/St_Thomas" => 208,
        "America/St_Vincent" => 209,
        "America/Swift_Current" => 210,
        "America/Tegucigalpa" => 211,
        "America/Thule" => 212,
        "America/Thunder_Bay" => 213,
        "America/Tijuana" => 214,
        "America/Toronto" => 215,
        "America/Tortola" => 216,
        "America/Vancouver" => 217,
        "America/Virgin" => 218,
        "America/Whitehorse" => 219,
        "America/Winnipeg" => 220,
        "America/Yakutat" => 221,
        "America/Yellowknife" => 222,
        "Antarctica/Casey" => 223,
        "Antarctica/Davis" => 224,
        "Antarctica/DumontDUrville" => 225,
        "Antarctica/Macquarie" => 226,
        "Antarctica/Mawson" => 227,
        "Antarctica/McMurdo" => 228,
        "Antarctica/Palmer" => 229,
        "Antarctica/Rothera" => 230,
        "Antarctica/South_Pole" => 231,
        "Antarctica/Syowa" => 232,
        "Antarctica/Troll" => 233,
        "Antarctica/Vostok" => 234,
        "Arctic/Longyearbyen" => 235,
        "Asia/Aden" => 236,
        "Asia/Almaty" => 237,
        "Asia/Amman" => 238,
        "Asia/Anadyr" => 239,
        "Asia/Aqtau" => 240,
        "Asia/Aqtobe" => 241,
        "Asia/Ashgabat" => 242,
        "Asia/Ashkhabad" => 243,
        "Asia/Atyrau" => 244,
        "Asia/Baghdad" => 245,
        "Asia/Bahrain" => 246,
        "Asia/Baku" => 247,
        "Asia/Bangkok" => 248,
        "Asia/Barnaul" => 249,
        "Asia/Beirut" => 250,
        "Asia/Bishkek" => 251,
        "Asia/Brunei" => 252,
        "Asia/Calcutta" => 253,
        "Asia/Chita" => 254,
        "Asia/Choibalsan" => 255,
        "Asia/Chongqing" => 256,
        "Asia/Chungking" => 257,
        "Asia/Colombo" => 258,
        "Asia/Dacca" => 259,
        "Asia/Damascus" => 260,
        "Asia/Dhaka" => 261,
        "Asia/Dili" => 262,
        "Asia/Dubai" => 263,
        "Asia/Dushanbe" => 264,
        "Asia/Famagusta" => 265,
        "Asia/Gaza" => 266,
        "Asia/Harbin" => 267,
        "Asia/Hebron" => 268,
        "Asia/Ho_Chi_Minh" => 269,
        "Asia/Hong_Kong" => 270,
        "Asia/Hovd" => 271,
        "Asia/Irkutsk" => 272,
        "Asia/Istanbul" => 273,
        "Asia/Jakarta" => 274,
        "Asia/Jayapura" => 275,
        "Asia/Jerusalem" => 276,
        "Asia/Kabul" => 277,
        "Asia/Kamchatka" => 278,
        "Asia/Karachi" => 279,
        "Asia/Kashgar" => 280,
        "Asia/Kathmandu" => 281,
        "Asia/Katmandu" => 282,
        "Asia/Khandyga" => 283,
        "Asia/Kolkata" => 284,
        "Asia/Krasnoyarsk" => 285,
        "Asia/Kuala_Lumpur" => 286,
        "Asia/Kuching" => 287,
        "Asia/Kuwait" => 288,
        "Asia/Macao" => 289,
        "Asia/Macau" => 290,
        "Asia/Magadan" => 291,
        "Asia/Makassar" => 292,
        "Asia/Manila" => 293,
        "Asia/Muscat" => 294,
        "Asia/Nicosia" => 295,
        "Asia/Novokuznetsk" => 296,
        "Asia/Novosibirsk" => 297,
        "Asia/Omsk" => 298,
        "Asia/Oral" => 299,
        "Asia/Phnom_Penh" => 300,
        "Asia/Pontianak" => 301,
        "Asia/Pyongyang" => 302,
        "Asia/Qatar" => 303,
        "Asia/Qostanay" => 304,
        "Asia/Qyzylorda" => 305,
        "Asia/Rangoon" => 306,
        "Asia/Riyadh" => 307,
        "Asia/Saigon" => 308,
        "Asia/Sakhalin" => 309,
        "Asia/Samarkand" => 310,
        "Asia/Seoul" => 311,
        "Asia/Shanghai" => 312,
        "Asia/Singapore" => 313,
        "Asia/Srednekolymsk" => 314,
        "Asia/Taipei" => 315,
        "Asia/Tashkent" => 316,
        "Asia/Tbilisi" => 317,
        "Asia/Tehran" => 318,
        "Asia/Tel_Aviv" => 319,
        "Asia/Thimbu" => 320,
        "Asia/Thimphu" => 321,
        "Asia/Tokyo" => 322,
        "Asia/Tomsk" => 323,
        "Asia/Ujung_Pandang" => 324,
        "Asia/Ulaanbaatar" => 325,
        "Asia/Ulan_Bator" => 326,
        "Asia/Urumqi" => 327,
        "Asia/Ust-Nera" => 328,
        "Asia/Vientiane" => 329,
        "Asia/Vladivostok" => 330,
        "Asia/Yakutsk" => 331,
        "Asia/Yangon" => 332,
        "Asia/Yekaterinburg" => 333,
        "Asia/Yerevan" => 334,
        "Atlantic/Azores" => 335,
        "Atlantic/Bermuda" => 336,
        "Atlantic/Canary" => 337,
        "Atlantic/Cape_Verde" => 338,
        "Atlantic/Faeroe" => 339,
        "Atlantic/Faroe" => 340,
        "Atlantic/Jan_Mayen" => 341,
        "Atlantic/Madeira" => 342,
        "Atlantic/Reykjavik" => 343,
        "Atlantic/South_Georgia" => 344,
        "Atlantic/St_Helena" => 345,
        "Atlantic/Stanley" => 346,
        "Australia/ACT" => 347,
        "Australia/Adelaide" => 348,
        "Australia/Brisbane" => 349,
        "Australia/Broken_Hill" => 350,
        "Australia/Canberra" => 351,
        "Australia/Currie" => 352,
        "Australia/Darwin" => 353,
        "Australia/Eucla" => 354,
        "Australia/Hobart" => 355,
        "Australia/LHI" => 356,
        "Australia/Lindeman" => 357,
        "Australia/Lord_Howe" => 358,
        "Australia/Melbourne" => 359,
        "Australia/NSW" => 360,
        "Australia/North" => 361,
        "Australia/Perth" => 362,
        "Australia/Queensland" => 363,
        "Australia/South" => 364,
        "Australia/Sydney" => 365,
        "Australia/Tasmania" => 366,
        "Australia/Victoria" => 367,
        "Australia/West" => 368,
        "Australia/Yancowinna" => 369,
        "Brazil/Acre" => 370,
        "Brazil/DeNoronha" => 371,
        "Brazil/East" => 372,
        "Brazil/West" => 373,
        "CET" => 374,
        "CST6CDT" => 375,
        "Canada/Atlantic" => 376,
        "Canada/Central" => 377,
        "Canada/Eastern" => 378,
        "Canada/Mountain" => 379,
        "Canada/Newfoundland" => 380,
        "Canada/Pacific" => 381,
        "Canada/Saskatchewan" => 382,
        "Canada/Yukon" => 383,
        "Chile/Continental" => 384,
        "Chile/EasterIsland" => 385,
        "Cuba" => 386,
        "EET" => 387,
        "EST" => 388,
        "EST5EDT" => 389,
        "Egypt" => 390,
        "Eire" => 391,
        "Etc/GMT" => 392,
        "Etc/GMT+0" => 393,
        "Etc/GMT+1" => 394,
        "Etc/GMT+10" => 395,
        "Etc/GMT+11" => 396,
        "Etc/GMT+12" => 397,
        "Etc/GMT+2" => 398,
        "Etc/GMT+3" => 399,
        "Etc/GMT+4" => 400,
        "Etc/GMT+5" => 401,
        "Etc/GMT+6" => 402,
        "Etc/GMT+7" => 403,
        "Etc/GMT+8" => 404,
        "Etc/GMT+9" => 405,
        "Etc/GMT-0" => 406,
        "Etc/GMT-1" => 407,
        "Etc/GMT-10" => 408,
        "Etc/GMT-11" => 409,
        "Etc/GMT-12" => 410,
        "Etc/GMT-13" => 411,
        "Etc/GMT-14" => 412,
        "Etc/GMT-2" => 413,
        "Etc/GMT-3" => 414,
        "Etc/GMT-4" => 415,
        "Etc/GMT-5" => 416,
        "Etc/GMT-6" => 417,
        "Etc/GMT-7" => 418,
        "Etc/GMT-8" => 419,
        "Etc/GMT-9" => 420,
        "Etc/GMT0" => 421,
        "Etc/Greenwich" => 422,
        "Etc/UCT" => 423,
        "Etc/UTC" => 424,
        "Etc/Universal" => 425,
        "Etc/Zulu" => 426,
        "Europe/Amsterdam" => 427,
        "Europe/Andorra" => 428,
        "Europe/Astrakhan" => 429,
        "Europe/Athens" => 430,
        "Europe/Belfast" => 431,
        "Europe/Belgrade" => 432,
        "Europe/Berlin" => 433,
        "Europe/Bratislava" => 434,
        "Europe/Brussels" => 435,
        "Europe/Bucharest" => 436,
        "Europe/Budapest" => 437,
        "Europe/Busingen" => 438,
        "Europe/Chisinau" => 439,
        "Europe/Copenhagen" => 440,
        "Europe/Dublin" => 441,
        "Europe/Gibraltar" => 442,
        "Europe/Guernsey" => 443,
        "Europe/Helsinki" => 444,
        "Europe/Isle_of_Man" => 445,
        "Europe/Istanbul" => 446,
        "Europe/Jersey" => 447,
        "Europe/Kaliningrad" => 448,
        "Europe/Kiev" => 449,
        "Europe/Kirov" => 450,
        "Europe/Kyiv" => 451,
        "Europe/Lisbon" => 452,
        "Europe/Ljubljana" => 453,
        "Europe/London" => 454,
        "Europe/Luxembourg" => 455,
        "Europe/Madrid" => 456,
        "Europe/Malta" => 457,
        "Europe/Mariehamn" => 458,
        "Europe/Minsk" => 459,
        "Europe/Monaco" => 460,
        "Europe/Moscow" => 461,
        "Europe/Nicosia" => 462,
        "Europe/Oslo" => 463,
        "Europe/Paris" => 464,
        "Europe/Podgorica" => 465,
        "Europe/Prague" => 466,
        "Europe/Riga" => 467,
        "Europe/Rome" => 468,
        "Europe/Samara" => 469,
        "Europe/San_Marino" => 470,
        "Europe/Sarajevo" => 471,
        "Europe/Saratov" => 472,
        "Europe/Simferopol" => 473,
        "Europe/Skopje" => 474,
        "Europe/Sofia" => 475,
        "Europe/Stockholm" => 476,
        "Europe/Tallinn" => 477,
        "Europe/Tirane" => 478,
        "Europe/Tiraspol" => 479,
        "Europe/Ulyanovsk" => 480,
        "Europe/Uzhgorod" => 481,
        "Europe/Vaduz" => 482,
        "Europe/Vatican" => 483,
        "Europe/Vienna" => 484,
        "Europe/Vilnius" => 485,
        "Europe/Volgograd" => 486,
        "Europe/Warsaw" => 487,
        "Europe/Zagreb" => 488,
        "Europe/Zaporozhye" => 489,
        "Europe/Zurich" => 490,
        "Factory" => 491,
        "GB" => 492,
        "GB-Eire" => 493,
        "GMT" => 494,
        "GMT+0" => 495,
        "GMT-0" => 496,
        "GMT0" => 497,
        "Greenwich" => 498,
        "HST" => 499,
        "Hongkong" => 500,
        "Iceland" => 501,
        "Indian/Antananarivo" => 502,
        "Indian/Chagos" => 503,
        "Indian/Christmas" => 504,
        "Indian/Cocos" => 505,
        "Indian/Comoro" => 506,
        "Indian/Kerguelen" => 507,
        "Indian/Mahe" => 508,
        "Indian/Maldives" => 509,
        "Indian/Mauritius" => 510,
        "Indian/Mayotte" => 511,
        "Indian/Reunion" => 512,
        "Iran" => 513,
        "Israel" => 514,
        "Jamaica" => 515,
        "Japan" => 516,
        "Kwajalein" => 517,
        "Libya" => 518,
        "MET" => 519,
        "MST" => 520,
        "MST7MDT" => 521,
        "Mexico/BajaNorte" => 522,
        "Mexico/BajaSur" => 523,
        "Mexico/General" => 524,
        "NZ" => 525,
        "NZ-CHAT" => 526,
        "Navajo" => 527,
        "PRC" => 528,
        "PST8PDT" => 529,
        "Pacific/Apia" => 530,
        "Pacific/Auckland" => 531,
        "Pacific/Bougainville" => 532,
        "Pacific/Chatham" => 533,
        "Pacific/Chuuk" => 534,
        "Pacific/Easter" => 535,
        "Pacific/Efate" => 536,
        "Pacific/Enderbury" => 537,
        "Pacific/Fakaofo" => 538,
        "Pacific/Fiji" => 539,
        "Pacific/Funafuti" => 540,
        "Pacific/Galapagos" => 541,
        "Pacific/Gambier" => 542,
        "Pacific/Guadalcanal" => 543,
        "Pacific/Guam" => 544,
        "Pacific/Honolulu" => 545,
        "Pacific/Johnston" => 546,
        "Pacific/Kanton" => 547,
        "Pacific/Kiritimati" => 548,
        "Pacific/Kosrae" => 549,
        "Pacific/Kwajalein" => 550,
        "Pacific/Majuro" => 551,
        "Pacific/Marquesas" => 552,
        "Pacific/Midway" => 553,
        "Pacific/Nauru" => 554,
        "Pacific/Niue" => 555,
        "Pacific/Norfolk" => 556,
        "Pacific/Noumea" => 557,
        "Pacific/Pago_Pago" => 558,
        "Pacific/Palau" => 559,
        "Pacific/Pitcairn" => 560,
        "Pacific/Pohnpei" => 561,
        "Pacific/Ponape" => 562,
        "Pacific/Port_Moresby" => 563,
        "Pacific/Rarotonga" => 564,
        "Pacific/Saipan" => 565,
        "Pacific/Samoa" => 566,
        "Pacific/Tahiti" => 567,
        "Pacific/Tarawa" => 568,
        "Pacific/Tongatapu" => 569,
        "Pacific/Truk" => 570,
        "Pacific/Wake" => 571,
        "Pacific/Wallis" => 572,
        "Pacific/Yap" => 573,
        "Poland" => 574,
        "Portugal" => 575,
        "ROC" => 576,
        "ROK" => 577,
        "Singapore" => 578,
        "Turkey" => 579,
        "UCT" => 580,
        "US/Alaska" => 581,
        "US/Aleutian" => 582,
        "US/Arizona" => 583,
        "US/Central" => 584,
        "US/East-Indiana" => 585,
        "US/Eastern" => 586,
        "US/Hawaii" => 587,
        "US/Indiana-Starke" => 588,
        "US/Michigan" => 589,
        "US/Mountain" => 590,
        "US/Pacific" => 591,
        "US/Samoa" => 592,
        "UTC" => 593,
        "Universal" => 594,
        "W-SU" => 595,
        "WET" => 596,
        "Zulu" => 597,
        _ => return None,
    })
}

pub fn name(ordinal: i32) -> Option<&'static str> {
    TIME_ZONES
        .get(usize::try_from(ordinal).ok()?)
        .map(|zone| zone.id)
}

#[cfg(test)]
mod tests {
    use super::{TIME_ZONE_COUNT, TIME_ZONES, name, ordinal};

    #[test]
    fn catalog_is_dense_and_round_trips() {
        assert_eq!(TIME_ZONES.len(), TIME_ZONE_COUNT);
        assert_eq!(ordinal("Europe/Paris"), Some(464));
        assert_eq!(name(464), Some("Europe/Paris"));
        assert_eq!(ordinal("Europe/NotARealZone"), None);
    }

    #[test]
    fn member_names_are_unique() {
        let mut members: Vec<_> = TIME_ZONES.iter().map(|zone| zone.member).collect();
        members.sort_unstable();
        members.dedup();
        assert_eq!(members.len(), TIME_ZONE_COUNT);
    }
}

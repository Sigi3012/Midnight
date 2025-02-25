use midnight_model::database::OsuGroup;
use poise::serenity_prelude::Colour;
use tokio::time::Duration;

// API
pub const OSU_BASE_URL: &str = "https://osu.ppy.sh/api/v2";
pub const OSU_TOKEN_GRANT_URL: &str = "https://osu.ppy.sh/oauth/token";
pub const SAFEBOORU_BASE_URL: &str =
    "https://safebooru.org/index.php?page=dapi&s=post&q=index&pid={page_id}&limit=100";
pub const THECATAPI_BASE_URL: &str = "https://api.thecatapi.com/v1/images/search";

// Error messages
pub const STICKY_ERROR_MESSAGE: &str = "I couldn't the reference message from your input. This could be due to a few reasons:\n\
                                        - The channel or message doesnt exist\n\
                                        - A valid link wasn't provided (Example link: `https://discord.com/channels/1044380103427244033/1326950497327779840/1327326146810875954`)";

// Times
pub const EMBED_BUTTON_TIMEOUT: Duration = Duration::from_secs(60 * 120);
pub const ERROR_BACKOFF_COOLDOWN: Duration = Duration::from_secs(60 * 3);
pub const GROUP_LOOP_DURATION: Duration = Duration::from_secs(60 * 60 * 4);
pub const MAPFEED_LOOP_DURATION: Duration = Duration::from_secs(60 * 15);

// Colours
pub const RED: Colour = Colour::new(0xff3737);
pub const GREEN: Colour = Colour::new(0x80ff80);
pub const YELLOW: Colour = Colour::new(0xffff80);

pub const RANKED_COLOUR: Colour = Colour::from_rgb(64, 90, 201);
pub const QUALIFIED_COLOUR: Colour = Colour::from_rgb(209, 160, 61);
pub const DISQUALIFIED_COLOUR: Colour = Colour::from_rgb(210, 43, 43);
pub const LOVED_COLOUR: Colour = Colour::from_rgb(255, 105, 180);

// Misc
pub const BOORU_PAGE_RANGE: std::ops::Range<i16> = 1..148;

pub const MAX_CONCURRENT_REQUESTS: usize = 16;
pub const TRACKED_OSU_GROUPS: [OsuGroup; 8] = [
    OsuGroup::BeatmapNominator,
    OsuGroup::ProbationaryBeatmapNominator,
    OsuGroup::NominationAssessmentTeam,
    OsuGroup::GlobalModerationTeam,
    OsuGroup::Developer,
    OsuGroup::FeatureArtist,
    OsuGroup::BeatmapSpotlightCurator,
    OsuGroup::ProjectLoved,
];

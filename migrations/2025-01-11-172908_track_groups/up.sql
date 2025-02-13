-- Your SQL goes here
CREATE TYPE osu_group AS ENUM (
    'BeatmapNominator',
    'ProbationaryBeatmapNominator',
    'NominationAssessmentTeam',
    'GlobalModerationTeam',
    'Developer',
    'FeatureArtist',
    'BeatmapSpotlightCurator',
    'ProjectLoved',
    'TournamentCommittee',
    'Alumni'
    );

CREATE TYPE osu_gamemode AS ENUM (
    'Standard',
    'Mania',
    'Taiko',
    'Fruits' -- Staying consistent with how osu! stores gamemodes
    );

CREATE TABLE osu_users
(
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    avatar_url TEXT NOT NULL
);

CREATE TABLE osu_user_groups
(
    id        SERIAL PRIMARY KEY,
    user_id   INTEGER   NOT NULL REFERENCES osu_users (id),
    member_of osu_group NOT NULL
);

CREATE TABLE osu_user_group_gamemodes
(
    id            SERIAL PRIMARY KEY,
    user_group_id INTEGER      NOT NULL REFERENCES osu_user_groups (id) ON DELETE CASCADE,
    gamemode      osu_gamemode NOT NULL
);

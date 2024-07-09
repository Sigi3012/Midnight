CREATE TYPE user_addition_status AS ENUM (
    'UserAdded',
    'UserAlreadyExists'
);

CREATE TYPE user_deletion_status AS ENUM (
    'UserRemoved',
    'UserDoesNotExist'
);

CREATE FUNCTION add_user_id_to_subscribed_users(user_id bigint, beatmapset_id integer)
RETURNS user_addition_status AS $$
DECLARE
    user_exists boolean;
BEGIN
    SELECT user_id = ANY(subscribed_user_ids)
    INTO user_exists
    FROM beatmapsets
    WHERE beatmapsets.beatmapset_id = $2;
    
    IF user_exists THEN
        RETURN 'UserAlreadyExists';
    ELSE
        UPDATE beatmapsets
        SET subscribed_user_ids = array_append(subscribed_user_ids, user_id)
        WHERE beatmapsets.beatmapset_id = $2;
        RETURN 'UserAdded';
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION remove_user_id_from_subscribed_users(user_id bigint, beatmapset_id integer)
RETURNS user_deletion_status AS $$
DECLARE
    user_exists boolean;
BEGIN
    SELECT user_id = ANY(subscribed_user_ids)
    INTO user_exists
    FROM beatmapsets
    WHERE beatmapsets.beatmapset_id = $2;

    IF user_exists THEN
        UPDATE beatmapsets
        SET subscribed_user_ids = array_remove(subscribed_user_ids, user_id)
        WHERE beatmapsets.beatmapset_id = $2;
        RETURN 'UserRemoved';
    ELSE
        RETURN 'UserDoesNotExist';
    END IF;
END;
$$ LANGUAGE plpgsql;


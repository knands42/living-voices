CREATE TRIGGER adventure_create_initial_state
    AFTER INSERT ON adventure
    FOR EACH ROW
BEGIN
    INSERT INTO state (id, adventure_id)
    VALUES (
        lower(
            hex(randomblob(4)) || '-' ||
            hex(randomblob(2)) || '-' ||
            '4' || substr(hex(randomblob(2)), 2) || '-' ||
            substr('89ab', abs(random()) % 4 + 1, 1) ||
            substr(hex(randomblob(2)), 2) || '-' ||
            hex(randomblob(6))
        ),
        NEW.id
    );
END;

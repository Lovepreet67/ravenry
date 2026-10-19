-- Add up migration script here
INSERT INTO roles (title, description, slug)
VALUES
('Admin','admin of system','admin'),
('User Manager', 'User manager of system','user_manager'),
('Member', 'Just a member','member');

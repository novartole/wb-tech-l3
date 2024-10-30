CREATE TABLE IF NOT EXISTS users (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100) NOT NULL,
  email VARCHAR(50) NOT NULL
);

CREATE TABLE IF NOT EXISTS products (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100) NOT NULL,
  price INTEGER NOT NULL
);

CREATE OR REPLACE FUNCTION users_change_notify() 
RETURNS TRIGGER AS $$
DECLARE
  payload JSON;
BEGIN
  IF TG_OP = 'INSERT' THEN
    payload = json_build_object(
      'operation_type', TG_OP,
      'user_id', NEW.id,
      'name_after', NEW.name,
      'email_after', NEW.email
    );
  ELSIF TG_OP = 'UPDATE' THEN
    payload = json_build_object(
      'operation_type', TG_OP,
      'user_id', NEW.id,
      'name_before', OLD.name,
      'name_after', NEW.name,
      'email_before', OLD.email,
      'email_after', NEW.email
    );
  ELSIF TG_OP = 'DELETE' THEN
    payload = json_build_object(
      'operation_type', TG_OP,
      'user_id', OLD.id,
      'name_before', OLD.name,
      'email_before', OLD.email
    );
  END IF;
  PERFORM pg_notify('users_change', payload::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS users_change_trigger ON users;
CREATE TRIGGER users_change_trigger
AFTER INSERT OR UPDATE OR DELETE
ON users
FOR EACH ROW
EXECUTE FUNCTION users_change_notify();

CREATE OR REPLACE FUNCTION products_change_notify() 
RETURNS TRIGGER AS $$
DECLARE
  payload JSON;
BEGIN
  IF TG_OP = 'INSERT' THEN
    payload = json_build_object(
      'operation_type', TG_OP,
      'product_id', NEW.id,
      'name_after', NEW.name,
      'price_after', NEW.price
    );
  ELSIF TG_OP = 'UPDATE' THEN
    payload = json_build_object(
      'operation_type', TG_OP,
      'product_id', NEW.id,
      'name_before', OLD.name,
      'name_after', NEW.name,
      'price_before', OLD.price,
      'price_after', NEW.price
    );
  ELSIF TG_OP = 'DELETE' THEN
    payload = json_build_object(
      'operation_type', TG_OP,
      'product_id', OLD.id,
      'user_id', OLD.id,
      'name_before', OLD.name,
      'price_before', OLD.price
    );
  END IF;
  PERFORM pg_notify('products_change', payload::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS products_change_trigger ON products;
CREATE TRIGGER products_change_trigger
AFTER INSERT OR UPDATE OR DELETE
ON products
FOR EACH ROW
EXECUTE FUNCTION products_change_notify();

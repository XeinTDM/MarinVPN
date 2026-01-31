-- Seed initial servers
INSERT OR IGNORE INTO vpn_servers (country, city, endpoint, public_key, current_load, avg_latency) VALUES 
('Sweden', 'Stockholm', 'se-sto.marinvpn.net:51820', 'se_pub_key', 12, 15),
('United States', 'New York', 'us-nyc.marinvpn.net:51820', 'us_pub_key', 45, 85),
('Germany', 'Frankfurt', 'de-fra.marinvpn.net:51820', 'de_pub_key', 22, 25),
('United Kingdom', 'London', 'gb-lon.marinvpn.net:51820', 'gb_pub_key', 31, 28),
('Netherlands', 'Amsterdam', 'nl-ams.marinvpn.net:51820', 'nl_pub_key', 18, 19);

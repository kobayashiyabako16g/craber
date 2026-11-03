-- A checkout against fixtures/tiny.sql, for `--flow`: one step per statement,
-- each touching a different part of the diagram so a playback visibly moves.

-- Who is buying, and whether their coupon is valid.
SELECT u.id, p.display_name, c.code
FROM public.users u
JOIN public.user_profiles p ON p.user_id = u.id
LEFT JOIN public.coupons c ON c.code = 'WELCOME'
WHERE u.email = 'alice@example.com';

INSERT INTO public.orders (id, user_id, coupon_id, status, total)
VALUES (100, 1, 7, 'pending', 0);

INSERT INTO public.order_items (order_id, product_id, quantity)
VALUES (100, 42, 2);

UPDATE public.orders o
SET total = 59.80, status = 'paid'
WHERE o.id = 100;

INSERT INTO public.order_receipts (id, order_id)
SELECT 900, o.id FROM public.orders o WHERE o.status = 'paid' AND o.id = 100;

-- The coupon is single-use.
DELETE FROM public.coupons WHERE id = 7;

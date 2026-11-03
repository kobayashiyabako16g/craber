-- Closing an account against fixtures/tiny.sql. The orders and profile go with
-- the user through ON DELETE CASCADE; the reads first are what support checks.

SELECT o.id, o.status, r.issued_at
FROM public.orders o
LEFT JOIN public.order_receipts r ON r.order_id = o.id
WHERE o.user_id = 1;

DELETE FROM public.order_items
WHERE order_id IN (SELECT id FROM public.orders WHERE user_id = 1);

DELETE FROM public.users WHERE id = 1;

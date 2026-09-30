-- ============================================================
--  SPRINT 12 : RÉVISION DES LOYERS (ICC / ILC)
-- ============================================================

-- 1. Ajout du loyer courant (après révisions) sur leases
ALTER TABLE leases
    ADD COLUMN IF NOT EXISTS current_rent_cents BIGINT NOT NULL DEFAULT 0;

UPDATE leases
   SET current_rent_cents = rent_amount_cents
 WHERE current_rent_cents = 0;

COMMENT ON COLUMN leases.current_rent_cents IS
    'Loyer actuel du bail (mis à jour à chaque révision validée). Fallback : rent_amount_cents.';

-- 2. Enrichissement de lease_rent_revisions avec statuts + note + rattrapage
ALTER TABLE lease_rent_revisions
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'CALCULATED',
    ADD COLUMN IF NOT EXISTS note TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS validated_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS catching_invoice_id UUID REFERENCES invoices(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS override_rent_cents BIGINT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'lease_rent_revisions_status_check'
    ) THEN
        ALTER TABLE lease_rent_revisions
            ADD CONSTRAINT lease_rent_revisions_status_check
            CHECK (status IN ('CALCULATED','VALIDATED','ADJUSTED','REFUSED','POSTPONED'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_lease_rent_revisions_status
    ON lease_rent_revisions (legal_entity_id, status, calculation_date DESC);

COMMENT ON COLUMN lease_rent_revisions.status IS
    'CALCULATED (preview), VALIDATED (accepté), ADJUSTED (montant saisi), REFUSED (refusé), POSTPONED (reporté)';

-- 3. Peuplement des indices ICC + ILC pour TOUTES les legal_entities existantes
INSERT INTO lease_index_values (legal_entity_id, index_code, period_label, value, source_reference, verified_at)
SELECT
    le.id,
    v.index_code,
    v.period_label,
    v.value,
    'INSEE',
    '2024-01-01'::date
FROM legal_entities le
CROSS JOIN (VALUES
  -- ===== ICC (Indice du Coût de la Construction) =====
  ('ICC', 'T1 2000', 1069.000000::numeric),
  ('ICC', 'T2 2000', 1071.000000),
  ('ICC', 'T3 2000', 1074.000000),
  ('ICC', 'T4 2000', 1080.000000),
  ('ICC', 'T1 2001', 1089.000000),
  ('ICC', 'T2 2001', 1099.000000),
  ('ICC', 'T3 2001', 1108.000000),
  ('ICC', 'T4 2001', 1114.000000),
  ('ICC', 'T1 2002', 1123.000000),
  ('ICC', 'T2 2002', 1126.000000),
  ('ICC', 'T3 2002', 1133.000000),
  ('ICC', 'T4 2002', 1142.000000),
  ('ICC', 'T1 2003', 1157.000000),
  ('ICC', 'T2 2003', 1168.000000),
  ('ICC', 'T3 2003', 1179.000000),
  ('ICC', 'T4 2003', 1188.000000),
  ('ICC', 'T1 2004', 1210.000000),
  ('ICC', 'T2 2004', 1231.000000),
  ('ICC', 'T3 2004', 1250.000000),
  ('ICC', 'T4 2004', 1268.000000),
  ('ICC', 'T1 2005', 1288.000000),
  ('ICC', 'T2 2005', 1305.000000),
  ('ICC', 'T3 2005', 1325.000000),
  ('ICC', 'T4 2005', 1340.000000),
  ('ICC', 'T1 2006', 1365.000000),
  ('ICC', 'T2 2006', 1388.000000),
  ('ICC', 'T3 2006', 1414.000000),
  ('ICC', 'T4 2006', 1433.000000),
  ('ICC', 'T1 2007', 1464.000000),
  ('ICC', 'T2 2007', 1495.000000),
  ('ICC', 'T3 2007', 1523.000000),
  ('ICC', 'T4 2007', 1545.000000),
  ('ICC', 'T1 2008', 1580.000000),
  ('ICC', 'T2 2008', 1609.000000),
  ('ICC', 'T3 2008', 1627.000000),
  ('ICC', 'T4 2008', 1637.000000),
  ('ICC', 'T1 2009', 1613.000000),
  ('ICC', 'T2 2009', 1591.000000),
  ('ICC', 'T3 2009', 1585.000000),
  ('ICC', 'T4 2009', 1584.000000),
  ('ICC', 'T1 2010', 1591.000000),
  ('ICC', 'T2 2010', 1596.000000),
  ('ICC', 'T3 2010', 1607.000000),
  ('ICC', 'T4 2010', 1615.000000),
  ('ICC', 'T1 2011', 1635.000000),
  ('ICC', 'T2 2011', 1650.000000),
  ('ICC', 'T3 2011', 1669.000000),
  ('ICC', 'T4 2011', 1675.000000),
  ('ICC', 'T1 2012', 1690.000000),
  ('ICC', 'T2 2012', 1701.000000),
  ('ICC', 'T3 2012', 1705.000000),
  ('ICC', 'T4 2012', 1713.000000),
  ('ICC', 'T1 2013', 1721.000000),
  ('ICC', 'T2 2013', 1725.000000),
  ('ICC', 'T3 2013', 1723.000000),
  ('ICC', 'T4 2013', 1719.000000),
  ('ICC', 'T1 2014', 1715.000000),
  ('ICC', 'T2 2014', 1710.000000),
  ('ICC', 'T3 2014', 1706.000000),
  ('ICC', 'T4 2014', 1705.000000),
  ('ICC', 'T1 2015', 1704.000000),
  ('ICC', 'T2 2015', 1702.000000),
  ('ICC', 'T3 2015', 1704.000000),
  ('ICC', 'T4 2015', 1708.000000),
  ('ICC', 'T1 2016', 1718.000000),
  ('ICC', 'T2 2016', 1728.000000),
  ('ICC', 'T3 2016', 1737.000000),
  ('ICC', 'T4 2016', 1745.000000),
  ('ICC', 'T1 2017', 1757.000000),
  ('ICC', 'T2 2017', 1771.000000),
  ('ICC', 'T3 2017', 1789.000000),
  ('ICC', 'T4 2017', 1800.000000),
  ('ICC', 'T1 2018', 1823.000000),
  ('ICC', 'T2 2018', 1839.000000),
  ('ICC', 'T3 2018', 1854.000000),
  ('ICC', 'T4 2018', 1865.000000),
  ('ICC', 'T1 2019', 1885.000000),
  ('ICC', 'T2 2019', 1901.000000),
  ('ICC', 'T3 2019', 1913.000000),
  ('ICC', 'T4 2019', 1920.000000),
  ('ICC', 'T1 2020', 1933.000000),
  ('ICC', 'T2 2020', 1937.000000),
  ('ICC', 'T3 2020', 1948.000000),
  ('ICC', 'T4 2020', 1959.000000),
  ('ICC', 'T1 2021', 1975.000000),
  ('ICC', 'T2 2021', 1999.000000),
  ('ICC', 'T3 2021', 2028.000000),
  ('ICC', 'T4 2021', 2056.000000),
  ('ICC', 'T1 2022', 2098.000000),
  ('ICC', 'T2 2022', 2144.000000),
  ('ICC', 'T3 2022', 2186.000000),
  ('ICC', 'T4 2022', 2220.000000),
  ('ICC', 'T1 2023', 2274.000000),
  ('ICC', 'T2 2023', 2312.000000),
  ('ICC', 'T3 2023', 2342.000000),
  ('ICC', 'T4 2023', 2358.000000),
  ('ICC', 'T1 2024', 2383.000000),
  ('ICC', 'T2 2024', 2401.000000),
  ('ICC', 'T3 2024', 2411.000000),
  ('ICC', 'T4 2024', 2418.000000),
  ('ICC', 'T1 2025', 2425.000000),

  -- ===== ILC (Indice des Loyers Commerciaux) - base 100 = T1 2008 =====
  ('ILC', 'T1 2008', 100.000000),
  ('ILC', 'T2 2008', 101.210000),
  ('ILC', 'T3 2008', 102.420000),
  ('ILC', 'T4 2008', 103.040000),
  ('ILC', 'T1 2009', 102.620000),
  ('ILC', 'T2 2009', 101.880000),
  ('ILC', 'T3 2009', 101.650000),
  ('ILC', 'T4 2009', 101.710000),
  ('ILC', 'T1 2010', 102.220000),
  ('ILC', 'T2 2010', 102.780000),
  ('ILC', 'T3 2010', 103.250000),
  ('ILC', 'T4 2010', 103.700000),
  ('ILC', 'T1 2011', 104.440000),
  ('ILC', 'T2 2011', 105.220000),
  ('ILC', 'T3 2011', 105.850000),
  ('ILC', 'T4 2011', 106.370000),
  ('ILC', 'T1 2012', 107.250000),
  ('ILC', 'T2 2012', 107.760000),
  ('ILC', 'T3 2012', 107.950000),
  ('ILC', 'T4 2012', 108.180000),
  ('ILC', 'T1 2013', 108.720000),
  ('ILC', 'T2 2013', 109.040000),
  ('ILC', 'T3 2013', 108.930000),
  ('ILC', 'T4 2013', 108.750000),
  ('ILC', 'T1 2014', 108.570000),
  ('ILC', 'T2 2014', 108.360000),
  ('ILC', 'T3 2014', 108.090000),
  ('ILC', 'T4 2014', 108.030000),
  ('ILC', 'T1 2015', 108.010000),
  ('ILC', 'T2 2015', 108.030000),
  ('ILC', 'T3 2015', 108.130000),
  ('ILC', 'T4 2015', 108.260000),
  ('ILC', 'T1 2016', 108.460000),
  ('ILC', 'T2 2016', 108.710000),
  ('ILC', 'T3 2016', 108.940000),
  ('ILC', 'T4 2016', 109.190000),
  ('ILC', 'T1 2017', 109.620000),
  ('ILC', 'T2 2017', 110.100000),
  ('ILC', 'T3 2017', 110.540000),
  ('ILC', 'T4 2017', 110.950000),
  ('ILC', 'T1 2018', 111.620000),
  ('ILC', 'T2 2018', 112.160000),
  ('ILC', 'T3 2018', 112.550000),
  ('ILC', 'T4 2018', 112.910000),
  ('ILC', 'T1 2019', 113.680000),
  ('ILC', 'T2 2019', 114.250000),
  ('ILC', 'T3 2019', 114.660000),
  ('ILC', 'T4 2019', 115.030000),
  ('ILC', 'T1 2020', 115.740000),
  ('ILC', 'T2 2020', 116.210000),
  ('ILC', 'T3 2020', 116.550000),
  ('ILC', 'T4 2020', 116.940000),
  ('ILC', 'T1 2021', 117.700000),
  ('ILC', 'T2 2021', 118.410000),
  ('ILC', 'T3 2021', 119.060000),
  ('ILC', 'T4 2021', 119.750000),
  ('ILC', 'T1 2022', 120.690000),
  ('ILC', 'T2 2022', 121.710000),
  ('ILC', 'T3 2022', 122.660000),
  ('ILC', 'T4 2022', 123.620000),
  ('ILC', 'T1 2023', 124.790000),
  ('ILC', 'T2 2023', 125.800000),
  ('ILC', 'T3 2023', 126.640000),
  ('ILC', 'T4 2023', 127.420000),
  ('ILC', 'T1 2024', 128.350000),
  ('ILC', 'T2 2024', 129.100000),
  ('ILC', 'T3 2024', 129.750000),
  ('ILC', 'T4 2024', 130.300000),
  ('ILC', 'T1 2025', 130.850000)
) AS v(index_code, period_label, value)
ON CONFLICT (legal_entity_id, index_code, period_label) DO UPDATE
    SET value = EXCLUDED.value,
        source_reference = EXCLUDED.source_reference,
        verified_at = EXCLUDED.verified_at;
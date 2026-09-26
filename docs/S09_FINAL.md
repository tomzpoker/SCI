# S09 FINAL — Trésorerie

S09 est livré comme migration additive `0019` et module Rust `src/treasury.rs`.

La prévision s’appuie sur le solde bancaire courant, les loyers contractuels issus des baux, les flux récurrents détectés et les investissements futurs. Les points et snapshots sont historisés.

Les scénarios d’investissement calculent des impacts pondérés sans changer la comptabilité réelle. Les hypothèses sont enregistrées comme événements de type `HYPOTHESIS`.

Les états sont stockés en anglais pour une représentation stable du domaine et traduits en français dans l’UI.

# S05 FINAL — DOCUMENTS & OCR

## Statut
COMPLETE CÔTÉ SOURCE / READY FOR REVIEW.

## Migrations
- 0015_documents_ocr.sql

## Sécurité documentaire
Les octets des fichiers ne sont pas stockés en PostgreSQL. La BDD conserve les métadonnées,
hash SHA-256, chemin relatif, statut, classification, extraction et journal documentaire.

## Pipeline
Import → hash → détection doublon → classification → OCR → extraction → contrôles → validation.

Le passage à `documents.extracted_data` intervient uniquement après validation lorsqu’une
validation est requise.

Les documents ne sont plus supprimés physiquement par l’API de compatibilité `delete_document` :
ils sont archivés et l’événement est conservé.

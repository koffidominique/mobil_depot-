# MASTER PROMPT — DEPOTPRO MOBILE PREMIUM

## Application mobile Android & iOS avec React Native + TypeScript + Rust

**Version du cahier de développement :** DEPOTPRO MOBILE PREMIUM V1
**Projet parent :** DEPOTPRO PREMIUM V3
**Cible :** Android et iOS
**Architecture :** React Native + TypeScript + Rust + SQLite + API Axum
**Intégration :** version ordinateur DEPOTPRO React + TypeScript + Tauri
**Marché prioritaire :** Côte d'Ivoire, puis Afrique francophone
**Langue principale :** Français (fr-CI)
**Devise :** FCFA (XOF)
**Mode de fonctionnement :** Offline-first avec synchronisation multi-appareils

---

# PARTIE 1 — MISSION ET OBJECTIF

Tu es un agent IA senior spécialisé en architecture logicielle, Rust, React Native, TypeScript, SQLite, Tauri, applications de gestion commerciale, synchronisation distribuée et développement mobile multiplateforme.

Ta mission est de concevoir et développer **DEPOTPRO MOBILE PREMIUM**, une véritable application mobile professionnelle permettant aux propriétaires et employés de dépôts de boissons de gérer leurs activités depuis un smartphone Android ou iOS.

Cette application mobile doit être le prolongement fonctionnel de DEPOTPRO PREMIUM Desktop.

Elle ne doit pas être un projet indépendant, une simple démonstration graphique ou une application de consultation des données.

Elle doit réellement permettre de gérer :

- Les produits et les catalogues de boissons.
- Les prix d'achat et de vente.
- Les ventes au détail et en gros.
- Les ventes par casier, carton, pack, bouteille et canette.
- Les commandes mixtes.
- Les bouteilles vides et les casiers plastiques retournables.
- Les comptes clients et les crédits.
- Les fournisseurs et les achats.
- Les retours d'emballages aux fournisseurs.
- Les stocks de boissons pleines et d'emballages vides.
- La caisse, les paiements et les dépenses.
- Les reçus, les bordereaux et les rapports.
- La synchronisation avec la version ordinateur.

L'application doit être suffisamment robuste pour être utilisée quotidiennement dans un véritable dépôt de boissons.

## 1.1 Objectif commercial

Construire un produit premium commercialisable sous forme de solution de gestion pour les dépôts de boissons en Côte d'Ivoire.

L'application doit être conçue pour fonctionner aussi bien :

- Dans un petit dépôt exploité par une seule personne.
- Dans un dépôt avec plusieurs employés.
- Dans un dépôt avec plusieurs téléphones et un ordinateur.
- Dans une entreprise possédant plusieurs points de vente.
- Dans une installation temporairement dépourvue de connexion Internet.

## 1.2 Règle fondamentale

**Un seul moteur métier, plusieurs interfaces.**

React Native ne doit pas réimplémenter les règles métier déjà présentes ou prévues dans Rust.

Le cœur Rust doit rester responsable des validations, calculs, transactions, mouvements de stock, soldes d'emballages, règles de crédit et opérations sensibles.

React Native est responsable de l'interface mobile, de la présentation des informations, de la navigation et des interactions utilisateur.

Le serveur Axum est responsable de la réception des opérations synchronisées, de leur validation côté serveur, de la cohérence globale et de la distribution des modifications aux autres appareils.

---

# PARTIE 2 — AUDIT OBLIGATOIRE DU PROJET EXISTANT

## 2.1 Dépôt de référence

Le projet parent est :

`https://github.com/koffidominique/ss_depot_rust.git`

La branche de référence indiquée est `main`.

La version de référence précédemment identifiée est `3.0.0`.

Ces informations sont à vérifier dans le dépôt avant tout développement. Ne suppose jamais que le dépôt est resté identique depuis le dernier audit.

## 2.2 Première mission : comprendre l'existant

Avant d'écrire du code, réalise un audit complet du dépôt.

Examine :

- L'organisation du workspace Rust.
- Le crate `depotpro-core`.
- Le crate `depotpro-server`.
- Le module `src-tauri`.
- Le frontend React et TypeScript.
- Les clients HTTP et Tauri.
- Les services métier.
- Les modèles et DTO.
- Les migrations SQLite.
- Les composants et écrans existants.
- Les mécanismes d'authentification.
- Les tests unitaires et end-to-end.
- Les scripts de compilation.
- La configuration CI/CD.

Identifie les fonctions métier déjà disponibles et celles qui restent à développer.

Ne recrée pas une fonction qui existe déjà sans raison technique documentée.

## 2.3 Protection du projet

Avant toute modification :

1. Inspecter `git status`.
2. Vérifier les branches existantes.
3. Identifier les fichiers modifiés et non suivis.
4. Vérifier les migrations déjà appliquées.
5. Identifier les bases de données et sauvegardes existantes.
6. Vérifier les tests et compilations de référence.
7. Établir un rapport de référence.

Créer une branche de travail dédiée, par exemple :

`feature/depotpro-mobile-premium`

Ne jamais écraser les modifications utilisateur.

Ne jamais supprimer une base SQLite existante.

Ne jamais modifier directement une base de production pour réaliser un test.

Ne jamais lancer une migration destructive sans stratégie de sauvegarde et de retour arrière.

Le développement mobile doit être progressif et isolé de la version ordinateur jusqu'à validation de l'intégration.

## 2.4 Architecture existante à rationaliser

L'architecture cible du projet parent comprend :

- React + TypeScript pour l'interface ordinateur.
- Tauri 2 pour les applications Windows et macOS.
- Rust pour les règles métier et les services locaux.
- Axum pour l'API centrale.
- SQLite pour les bases locales.
- SQLx pour les accès SQL du backend lorsque cela est compatible avec l'architecture retenue.

Le dépôt a précédemment été identifié comme contenant également un backend Node/Express historique.

Ne conserve pas deux implémentations concurrentes des règles métier.

Le backend Node/Express ne doit pas être livré comme serveur métier final. Toutefois, sa suppression ne peut intervenir qu'après migration complète des fonctionnalités nécessaires vers Rust, vérification de la parité et réussite des tests.

Node.js peut rester présent comme outil de compilation du frontend, de gestion de dépendances ou d'outillage React Native. Il ne doit pas devenir un serveur métier concurrent.

---

# PARTIE 3 — ARCHITECTURE TECHNIQUE CIBLE

## 3.1 Stack mobile obligatoire

### Application mobile

- React Native.
- TypeScript strict.
- React Native New Architecture.
- Turbo Native Modules pour les connexions natives personnalisées.
- React Navigation pour la navigation.
- TanStack Query pour la gestion des requêtes et du cache.
- Zustand pour les états d'interface.
- Zod pour la validation des formulaires et contrats côté interface.
- Une solution de stockage sécurisé native compatible Android et iOS.
- Une solution de détection de connectivité réseau.

### Moteur métier

Rust doit être utilisé pour :

- Les règles de gestion.
- Les calculs commerciaux.
- Les validations.
- La gestion des stocks.
- Les mouvements d'emballages.
- Les écritures de caisse.
- Les opérations de crédit.
- Les transactions métier.
- La préparation des commandes de synchronisation.
- La validation des commandes entrantes du serveur.

### Serveur

Axum est le seul serveur HTTP métier.

Il fournit les services de :

- Authentification.
- Gestion des établissements et points de vente.
- Synchronisation.
- Gestion des utilisateurs et rôles.
- Réception des opérations mobiles et ordinateur.
- Distribution des modifications.
- Gestion des conflits.
- Historique des opérations.
- Administration et supervision.

### Base de données

SQLite est utilisé pour les bases locales.

Le serveur possède sa propre base centrale, distincte des fichiers SQLite de chaque appareil.

Ne jamais tenter de synchroniser directement le même fichier SQLite entre plusieurs téléphones et ordinateurs.

Chaque appareil possède sa propre base locale et communique avec le serveur central par API.

## 3.2 Intégration & Commandes natives

Le module métier doit exposer des commandes clairement identifiées :

- `initializeDatabase`
- `authenticate`
- `getDashboard`
- `createSale`
- `validateSale`
- `createMixedOrder`
- `registerPackagingReturn`
- `createPurchaseOrder`
- `receivePurchase`
- `getInventory`
- `getCustomerAccount`
- `getSupplierAccount`
- `recordPayment`
- `getPendingOperations`
- `startSynchronization`
- `getSynchronizationStatus`

---

# PARTIE 4 — FONCTIONNEMENT OFFLINE-FIRST

## 4.1 Principe

L'application doit rester utilisable même sans connexion Internet :

- Consultation des produits synchronisés.
- Consultation des prix disponibles.
- Consultation du stock local connu.
- Création d'une vente.
- Création d'une commande mixte.
- Enregistrement d'un retour d'emballage.
- Enregistrement d'un paiement.
- Création d'une commande fournisseur.
- Réception d'un achat selon les autorisations locales.
- Consultation des comptes locaux.
- Consultation des reçus déjà créés.
- Consultation des rapports calculables à partir des données locales.

Une opération réalisée hors ligne doit être enregistrée durablement avant de confirmer son succès à l'utilisateur.

## 4.2 Identité appareil et file `sync_outbox`

Prévoir : `tenant_id`, `branch_id`, `device_id`, `user_id`, `operation_id`, `local_sequence`, `server_sequence`, `schema_version`, `idempotency_key`.

Statuts `sync_outbox` : `PENDING`, `SYNCING`, `SYNCED`, `CONFLICT`, `FAILED`, `CANCELLED`.

Retry avec backoff exponentiel et variation aléatoire. Synchronisation montante idempotente, descendante par curseur serveur, gestion explicite des conflits sans écrasement silencieux des stocks, crédits ou emballages.

---

# PARTIE 5 — MODÈLE MÉTIER DU DÉPÔT DE BOISSONS

1. **Catalogue & Variantes** : Nom, marque, catégorie, SKU, code-barres, contenance, contenant, matière (verre, PET, aluminium, autre), statut actif/archivé. Ne jamais supposer la compatibilité physique par le volume seul.
2. **Unités commerciales** : Casier, Carton, Bouteille, Pack, Canette, Unité, Demi-casier (`capacity / 2` entier uniquement).
3. **Séparation des emballages** :
   - Stock de boissons pleines
   - Stock de bouteilles vides réutilisables
   - Stock de casiers plastiques réutilisables
   - Stock d'emballages jetables (PET, canettes, films, cartons — pas de dette d'emballage automatique).
4. **Casiers (`crate_types`)** : `B24-33`, `B12-50`, `B12-60`, `B12-65`, `B12-75`, `B12-100`, `B20-50`, `B6-150`, avec capacité, profil physique, groupe de compatibilité, valeur de remplacement/consigne optionnelle.
5. **Commandes mixtes** : Composer un casier compatible (ex. 6 Drogba 50 cl + 6 Beaufort 50 cl dans un `B12-50` = `12 / 12 bouteilles`), interdire dépassement de capacité, mélange incompatible, quantités fractionnaires.
6. **Retours partiels** : Ex. 10 bouteilles rendues sur 12 = 2 restantes dues, statut `PARTIEL`, sans facturation automatique sauf règle configurée.
7. **Comptes emballages immuables** : Registre `partner_packaging_ledger` historisé et immuable (corrections par écriture inverse), séparé des comptes financiers.

---

# PARTIE 6 — MODULES ET ÉCRANS MOBILES

1. **Navigation principale (5 entrées)** : Accueil, Ventes, Stock, Clients, Menu (Achats, Fournisseurs, Caisse, Rapports, Catalogue, Synchronisation & Paramètres).
2. **Connexion** : Logo DEPOTPRO, établissement, utilisateur, PIN/mot de passe, point de vente, durée de session & révocation.
3. **Tableau de bord** : CA du jour, ventes, encaissements, dépenses, crédits, créances, dettes fournisseurs, valeur stock, alertes rupture/stock bas/emballages, activité récente, état sync.
4. **Vente rapide (POS)** : Recherche, marque, catégorie, contenance, contenant, favoris, panier, client, tarif détail/gros, remise autorisée, calcul entier en FCFA, paiement, validation.
5. **Commande mixte** : Assistant étape par étape avec jauge `X / Y bouteilles`.
6. **Paiement** : Espèces, Mobile Money (confirmation requise), Virement, Chèque, Mixte, Partiel, Crédit.
7. **Reçu client** : Lignes de boissons + section distincte emballages (attendus, reçus, manquants) + export PDF / partage.
8. **Clients & Fournisseurs** : Comptes Financier et Emballages strictement séparés.
9. **Achats & Réception** : Bon de commande, réception partielle/totale, écarts, endommagés, retours emballages fournisseur.
10. **Stock** : 4 vues séparées + inventaire physique & ajustement justifié.
11. **Caisse & Finances** : Ouverture, fond, encaissements, décaissements, dépenses, clôture, écart.
12. **Rapports** : Ventes, marge estimée (distincte du CA), encaissements, dépenses, créances, dettes, emballages.
13. **Synchronisation & Appareils** : État connexion, appareil, outbox, conflits, révocation.

---

# PARTIE 7–17 — PLAN DE DÉVELOPPEMENT PAR LOTS, QUALITÉ ET RÈGLES STRICTES

- **LOT 0** : Audit et état initial
- **LOT 1** : Prototype socle métier + SQLite local + transactions
- **LOT 2** : Architecture commune, DTO, contrats API, permissions
- **LOT 3** : Base locale, migrations, index, sauvegarde/restauration
- **LOT 4** : Authentification, sessions, établissement, point de vente, appareil
- **LOT 5** : Synchronisation (Outbox, Push/Pull, curseurs, idempotence, conflits)
- **LOT 6** : Catalogue et prix
- **LOT 7** : Ventes et caisse
- **LOT 8** : Clients et emballages
- **LOT 9** : Achats et fournisseurs
- **LOT 10** : Stock, finances et rapports
- **LOT 11** : Design Premium (thème clair/sombre, responsive, accessibilité, animations)
- **LOT 12 & 13** : Intégration, tests automatisés, sécurité et validation finale

Règles critiques : zéro flottant pour les montants FCFA, zéro quantité fractionnaire de bouteilles, zéro suppression silencieuse de mouvements historiques, zéro confirmation de vente avant durabilité locale, zéro masquage de conflit de synchronisation.

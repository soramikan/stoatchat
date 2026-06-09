# Stoat ビルド・デプロイ・プッシュ通知設定手順

このワークスペースは複数リポジトリで構成されています。

- `stoatchat`: API、WebSocket、ファイル/プロキシサービス、`pushd`
- `stoat-web`: Web クライアント
- `stoat-android`: Android クライアント
- `stoat-ios`: iOS クライアント
- `stoat-desktop`: Electron デスクトップクライアント

既定の接続先はクライアント側で `https://chat.setoka.net` に向けています。Web の API は `https://chat.setoka.net/api`、WebSocket は `wss://chat.setoka.net/events` を使います。

## 1. サーバーと pushd

### ローカル開発

```sh
cd stoatchat
mise install
mise build
mise start
```

停止する場合:

```sh
mise docker:stop
```

テスト:

```sh
cd stoatchat
docker compose up -d
TEST_DB=REFERENCE cargo nextest run
TEST_DB=MONGODB cargo nextest run
```

### Docker イメージ

公式のスクリプトで各サービスのイメージを作成、push できます。

```sh
cd stoatchat
scripts/publish-debug-image.sh 20260609-1 false
```

このスクリプトは `server`、`bonfire`、`autumn`、`january`、`gifbox`、`crond`、`pushd`、`voice-ingress` を `ghcr.io/stoatchat/*` に push します。別レジストリへ出す場合はスクリプト内のタグを環境に合わせて変更してください。

### プッシュ通知サーバー設定

`pushd` は RabbitMQ 経由で通知イベントを受け取り、購読セッションの `endpoint` に応じて APNs、FCM、Web Push へ配送します。

本番設定は `Revolt.overrides.toml` で上書きしてください。最低限必要な設定例:

```toml
[hosts]
app = "https://chat.setoka.net"
api = "https://chat.setoka.net"
events = "wss://chat.setoka.net/events"
autumn = "https://cdn.stoatusercontent.com"
january = "https://proxy.stoatusercontent.com"

[pushd]
production = true

[pushd.apn]
queue = "notifications.outbound.apn"
sandbox = false
topic = "dev.mikanbox.stoat"
pkcs8 = "<Apple の .p8 キーを base64 エンコードした文字列>"
key_id = "<APNs Key ID>"
team_id = "<Apple Developer Team ID>"

[pushd.fcm]
queue = "notifications.outbound.fcm"
key_type = "service_account"
project_id = "<Firebase project_id>"
private_key_id = "<Firebase private_key_id>"
private_key = "<Firebase private_key>"
client_email = "<Firebase client_email>"
client_id = "<Firebase client_id>"
auth_uri = "https://accounts.google.com/o/oauth2/auth"
token_uri = "https://oauth2.googleapis.com/token"
auth_provider_x509_cert_url = "https://www.googleapis.com/oauth2/v1/certs"
client_x509_cert_url = "<Firebase client_x509_cert_url>"
```

APNs の `topic` は iOS アプリ本体の Bundle Identifier と一致させます。現在の iOS アプリは `dev.mikanbox.stoat`、通知サービス拡張は `dev.mikanbox.stoat.notifications` です。

FCM は Firebase のサービスアカウント JSON の各項目を `[pushd.fcm]` に転記します。`auth_uri` が空の場合、`pushd` は FCM outbound consumer を起動しません。

## 2. iOS

### 必要な Apple 設定

Apple Developer で以下を有効にします。

- App ID: `dev.mikanbox.stoat`
- Extension App ID: `dev.mikanbox.stoat.notifications`
- Capability: Push Notifications
- Capability: App Groups、必要に応じて `group.dev.mikanbox.stoat`
- APNs Auth Key: `pushd.apn.pkcs8`、`key_id`、`team_id` に設定

`Stoat/Stoat.entitlements` には `aps-environment` を追加済みです。開発ビルドは `development`、配布ビルドは Push Notifications を含む配布用 provisioning profile を使ってください。

### ローカルビルド

Simulator 向け:

```sh
cd stoat-ios
xcodebuild -project Stoat.xcodeproj \
  -scheme Stoat \
  -destination 'platform=iOS Simulator,name=iPhone 17,OS=26.5' \
  -skipMacroValidation \
  CODE_SIGNING_ALLOWED=NO \
  build
```

実機/Archive:

```sh
cd stoat-ios
xcodebuild -scheme Stoat \
  -skipPackagePluginValidation \
  -skipMacroValidation \
  -allowProvisioningUpdates \
  -archivePath /tmp/Stoat.xcarchive \
  -sdk iphoneos \
  -configuration Release \
  -destination generic/platform=iOS \
  clean archive
```

IPA export:

```sh
xcodebuild -exportArchive \
  -allowProvisioningUpdates \
  -archivePath /tmp/Stoat.xcarchive \
  -exportOptionsPlist /path/to/ExportOptions.plist \
  -exportPath /tmp/StoatExport
```

### iOS プッシュ通知の動作

ユーザーが通知設定で「Enable push notifications」を有効にすると、APNs token を取得し、API の `POST /push/subscribe` に以下の形式で登録します。

```json
{
  "endpoint": "apn",
  "p256dh": "",
  "auth": "<APNs device token>"
}
```

サーバーの `pushd` は `endpoint = "apn"` の購読へ APNs 通知を送ります。メッセージ通知には `ALERT_MESSAGE` category と `serverId` が入り、通知拡張と返信アクションが動作します。

## 3. Android

### 必要な Firebase 設定

Firebase プロジェクトで Android アプリを登録し、`google-services.json` を `stoat-android/app/google-services.json` に配置します。

現在の Gradle 設定の `applicationId` は以下です。

- Debug: `chat.revolt.debug`
- Release: `chat.revolt`

Firebase 側でも両方を登録しておくと、Debug/Release の両方で FCM token が取得できます。

### ローカルビルド

```sh
cd stoat-android
./gradlew :app:assembleDebug
./gradlew :app:compileDebugKotlin
```

Release APK/AAB:

```sh
cd stoat-android
./gradlew :app:assembleRelease
./gradlew :app:bundleRelease
```

このリポジトリには release signingConfig が定義されていないため、配布用署名は Android Studio、CI、または別途追加した Gradle signingConfig で行います。

### Android プッシュ通知の動作

アプリは Firebase Messaging token を取得し、ログイン済みセッションで API の `POST /push/subscribe` に以下の形式で登録します。

```json
{
  "endpoint": "fcm",
  "p256dh": "",
  "auth": "<FCM registration token>"
}
```

Android 13 以降では `POST_NOTIFICATIONS` 権限が必要です。アプリ内の通知設定画面で有効化すると、権限要求、FCM token 取得、サーバー登録を行います。サーバー登録が失敗した場合はローカル token を有効扱いにしません。

## 4. Web

### ローカル開発

```sh
cd stoat-web
git submodule update --init packages/stoat.js packages/solid-livekit-components packages/js-lingui-solid
mise install:frozen
mise build:deps
cp packages/client/.env.example packages/client/.env
mise dev
```

### ビルドとデプロイ

```sh
cd stoat-web
mise install:frozen
mise build:deps
mise build
```

本番向け:

```sh
mise build:prod
```

生成物は `stoat-web/packages/client/dist` です。`/login`、`/pwa`、`/dev`、`/discover`、`/settings`、`/invite`、`/bot`、`/friends`、`/server`、`/channel` を SPA fallback で配信してください。

## 5. Desktop

### ローカル開発

```sh
cd stoat-desktop
corepack pnpm install --frozen-lockfile
corepack pnpm start
```

開発用 Web サーバーへ接続する場合:

```sh
corepack pnpm start -- --force-server http://localhost:5173
```

### パッケージ作成

```sh
cd stoat-desktop
corepack pnpm package
corepack pnpm make
```

既定の起動先は `https://chat.setoka.net` です。別サーバーへ向ける場合は起動時に `--force-server <URL>` を渡します。

## 6. 確認手順

### サーバー

1. `pushd` のログに APNs/FCM outbound consumer が起動していることを確認します。
2. `POST /push/subscribe` 後、Authifier のセッションに `subscription.endpoint` が保存されていることを確認します。
3. DM または mention を送信し、RabbitMQ の `revolt.notifications` exchange から `notifications.outbound.apn` / `notifications.outbound.fcm` へ配送されることを確認します。

### iOS

1. 実機で通知権限を許可します。Simulator では APNs device token の実機配送検証はできません。
2. `didRegisterForRemoteNotificationsWithDeviceToken` が呼ばれ、`/push/subscribe` が成功することをログで確認します。
3. バックグラウンド状態で DM または mention を受け取り、通知表示、タップ遷移、返信アクションを確認します。

### Android

1. Google Play services が使える端末/エミュレータで通知権限を許可します。
2. FCM token が取得され、`/push/subscribe` が成功することを確認します。
3. バックグラウンド状態で DM または mention を受け取り、通知表示、タップ遷移、返信、既読アクションを確認します。

## 7. よくある失敗

- iOS に届かない: `pushd.apn.topic` と app bundle ID が一致していない、APNs key/team/key id が違う、sandbox/production が provisioning profile と合っていない。
- Android に届かない: `google-services.json` の package name が Gradle の `applicationId` と合っていない、Firebase service account を `pushd.fcm` に設定していない、端末に Google Play services がない。
- 通知設定を有効にしても届かない: クライアントでログイン済みセッションの token を登録できていない。`POST /push/subscribe` のレスポンスとサーバー側セッションの `subscription` を確認する。
- Web/Android/iOS で別サーバーへ向いている: 各クライアントの既定 URL または `.env` / `--force-server` /ビルド設定が `https://chat.setoka.net` になっているか確認する。

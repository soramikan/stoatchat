---
sidebar_position: 4
---

# Bot

Stoat の Bot は通常ユーザーとは別の Bot ユーザーとして動作します。Bot トークンを `X-Bot-Token` ヘッダーで渡すと、Bot が参加しているチャンネルにメッセージを送信したり、イベントサーバーへ接続してリアルタイムイベントを受け取ったりできます。

## 認証

API では以下のヘッダーを使います。

```http
X-Bot-Token: BOT_TOKEN
```

イベントサーバーでは `Authenticate` メッセージの `token` に Bot トークンを渡します。Bot はイベント接続時に Bot が見える範囲のイベントを受け取ります。

```json
{
  "type": "Authenticate",
  "token": "BOT_TOKEN"
}
```

トークンを再生成すると古いトークンは無効になり、イベントでは `Logout` が送信されます。

## Bot 管理 API

Bot 管理 API は Bot の所有者のユーザーセッショントークンで呼び出します。Bot トークンでは呼び出せません。

| 機能          | メソッド | パス                    | 説明                                                              |
| ------------- | -------- | ----------------------- | ----------------------------------------------------------------- |
| 作成          | `POST`   | `/bots/create`          | Bot ユーザーを作成します。                                        |
| 所有 Bot 一覧 | `GET`    | `/bots/@me`             | 自分が所有する Bot と対応する User を取得します。                 |
| 所有 Bot 取得 | `GET`    | `/bots/{bot_id}`        | 自分が所有する Bot の詳細を取得します。                           |
| 公開 Bot 取得 | `GET`    | `/bots/{bot_id}/invite` | 公開 Bot、または自分が所有する非公開 Bot の招待情報を取得します。 |
| 招待          | `POST`   | `/bots/{bot_id}/invite` | Bot をサーバーまたはグループに追加します。                        |
| 編集          | `PATCH`  | `/bots/{bot_id}`        | Bot 名、公開設定、分析設定、Interactions URL を変更します。       |
| 削除          | `DELETE` | `/bots/{bot_id}`        | Bot を削除します。                                                |

### Bot を作成する

```http
POST /bots/create
X-Session-Token: USER_SESSION_TOKEN
Content-Type: application/json
```

```json
{
  "name": "Release Bot"
}
```

レスポンスには `bot` と対応する `user` が含まれます。Bot トークンは `bot.token` として返されます。トークンは秘密情報として扱い、クライアントアプリに埋め込まないでください。

### Bot を編集する

```http
PATCH /bots/01H...
X-Session-Token: USER_SESSION_TOKEN
Content-Type: application/json
```

```json
{
  "name": "Release Bot",
  "public": true,
  "analytics": false,
  "interactions_url": "https://example.com/stoat/interactions"
}
```

`remove` を使うと値を削除できます。`Token` を指定するとトークンが再生成されます。

```json
{
  "remove": ["Token"]
}
```

### Bot を招待する

サーバーに招待するには、呼び出しユーザーが対象サーバーで `ManageServer` 権限を持っている必要があります。

```http
POST /bots/01H.../invite
X-Session-Token: USER_SESSION_TOKEN
Content-Type: application/json
```

```json
{
  "type": "Server",
  "server": "01H..."
}
```

グループに招待するには、呼び出しユーザーが対象グループで `InviteOthers` 権限を持っている必要があります。

```json
{
  "type": "Group",
  "group": "01H..."
}
```

## メッセージ送信

Bot がメッセージを送るには、対象チャンネルに参加していて `SendMessage` 権限を持っている必要があります。

```http
POST /channels/{channel_id}/messages
X-Bot-Token: BOT_TOKEN
Idempotency-Key: 01J...
Content-Type: application/json
```

```json
{
  "content": "デプロイが完了しました。"
}
```

`Idempotency-Key` は重複送信防止に使われます。再試行する可能性がある Bot は、送信試行ごとに同じキーを再利用してください。

### 添付ファイル

ファイルは Autumn にアップロードしてから、その attachment id を `attachments` に指定します。Bot トークンでもアップロードできます。

```http
POST /attachments
X-Bot-Token: BOT_TOKEN
```

```json
{
  "content": "ログを添付しました。",
  "attachments": ["01H..."]
}
```

添付には対象チャンネルの `UploadFiles` 権限が必要です。

### Masquerade

Bot や Webhook は `masquerade` を使って、メッセージごとの表示名・アイコン・色を上書きできます。対象チャンネルで `Masquerade` 権限が必要です。`colour` を使う場合は追加で `ManageRole` 権限が必要です。

```json
{
  "content": "外部サービスからの通知です。",
  "masquerade": {
    "name": "GitHub",
    "avatar": "https://github.githubassets.com/favicons/favicon.png",
    "colour": "#24292f"
  }
}
```

### Interactions

`interactions` を指定すると、メッセージで利用可能なリアクションを制御できます。対象チャンネルで `React` 権限が必要です。

```json
{
  "content": "承認しますか？",
  "interactions": {
    "reactions": ["👍", "👎"],
    "restrict_reactions": true
  }
}
```

### 通知抑制

`flags` に `1` を設定すると、プッシュ通知やデスクトップ通知を抑制できます。

```json
{
  "content": "夜間バッチが完了しました。",
  "flags": 1
}
```

## Discord 互換 Embed

`embeds` には既存の Stoat text embed と Discord 形式に近い rich embed を指定できます。対象チャンネルで `SendEmbeds` 権限が必要です。

```json
{
  "content": "ビルド結果",
  "embeds": [
    {
      "title": "Build #248 succeeded",
      "description": "main ブランチの Linux / macOS / Windows ビルドが成功しました。",
      "url": "https://example.com/builds/248",
      "color": 5763719,
      "timestamp": "2026-06-12T10:15:00Z",
      "author": {
        "name": "CI",
        "url": "https://example.com/ci",
        "icon_url": "https://example.com/ci.png"
      },
      "thumbnail": {
        "url": "https://example.com/thumbnail.png"
      },
      "image": {
        "url": "https://example.com/screenshot.png"
      },
      "video": {
        "url": "https://example.com/demo.mp4",
        "width": 1280,
        "height": 720
      },
      "provider": {
        "name": "CI Dashboard",
        "url": "https://example.com"
      },
      "fields": [
        {
          "name": "Commit",
          "value": "`f100c42f`",
          "inline": true
        },
        {
          "name": "Duration",
          "value": "4m 12s",
          "inline": true
        }
      ],
      "footer": {
        "text": "Stoat Bot",
        "icon_url": "https://example.com/footer.png"
      }
    }
  ]
}
```

対応フィールド:

| フィールド        | 型             | 制限                                                                       |
| ----------------- | -------------- | -------------------------------------------------------------------------- |
| `title`           | string         | 1-256 文字                                                                 |
| `description`     | string         | 1-4096 文字                                                                |
| `url`             | string         | 1-2048 文字                                                                |
| `timestamp`       | ISO8601 string | 例: `2026-06-12T10:15:00Z`                                                 |
| `color`           | integer        | `0` から `16777215` の RGB 値                                              |
| `colour`          | string         | Stoat 互換の CSS 色。指定がない場合、`color` から `#rrggbb` を生成します。 |
| `author.name`     | string         | 1-256 文字                                                                 |
| `author.url`      | string         | 1-2048 文字                                                                |
| `author.icon_url` | string         | 1-2048 文字                                                                |
| `footer.text`     | string         | 1-2048 文字                                                                |
| `footer.icon_url` | string         | 1-2048 文字                                                                |
| `fields`          | array          | 最大 25 件                                                                 |
| `fields[].name`   | string         | 1-256 文字                                                                 |
| `fields[].value`  | string         | 1-1024 文字                                                                |
| `fields[].inline` | boolean        | 省略時は `false`                                                           |
| `image.url`       | string         | 1-2048 文字                                                                |
| `thumbnail.url`   | string         | 1-2048 文字                                                                |
| `video.url`       | string         | 1-2048 文字                                                                |
| `provider.name`   | string         | 1-256 文字                                                                 |
| `provider.url`    | string         | 1-2048 文字                                                                |
| `media`           | attachment id  | 既存 Stoat 互換。Autumn の attachment id を指定します。                    |
| `icon_url`        | string         | 既存 Stoat 互換の小アイコン URL                                            |

1 メッセージに含められる embed 数はインスタンス設定の `features.limits.global.message_embeds` に従います。初期設定では 5 件です。embed 内の `title`、`description`、`author.name`、`footer.text`、`fields[].name`、`fields[].value` の合計は Discord と同じく 6000 文字までです。

## メッセージ編集・削除

Bot は自分が送信したメッセージを編集・削除できます。

```http
PATCH /channels/{channel_id}/messages/{message_id}
X-Bot-Token: BOT_TOKEN
Content-Type: application/json
```

```json
{
  "content": "内容を更新しました。",
  "embeds": []
}
```

```http
DELETE /channels/{channel_id}/messages/{message_id}
X-Bot-Token: BOT_TOKEN
```

## Webhook との使い分け

Webhook はチャンネル単位の固定トークンで外部サービスからメッセージを送る用途に向いています。Bot はサーバーやグループへ参加し、イベントを購読し、複数チャンネルで状態を持って動作する用途に向いています。

Webhook でも `DataMessageSend` を使うため、Discord 互換 embed は同じ payload で送信できます。

```http
POST /webhooks/{webhook_id}/{token}
Content-Type: application/json
```

```json
{
  "embeds": [
    {
      "title": "Webhook notification",
      "description": "Webhook からも rich embed を送信できます。",
      "color": 3447003
    }
  ]
}
```

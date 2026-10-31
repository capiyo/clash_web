# Firebase Storage for feed and profile images

The API now stores **new feed images and profile avatars** in the Firebase Storage bucket used by the web app:

- Firebase project: `funzy-d56d7`
- Bucket: `funzy-d56d7.firebasestorage.app`
- Feed object prefix: `posts/{userId}/`
- Profile object prefix: `profile-avatars/{userId}/`

The existing multipart API contracts remain unchanged. Web and React Native continue sending the selected image to the API; the backend validates it, uploads it to Firebase Storage, and persists the Firebase download URL and object path in MongoDB. This keeps the upload implementation consistent across platforms without requiring platform-specific Storage SDKs.

## Required backend secrets

Create a dedicated service account in the **funzy-d56d7** Google/Firebase project. Grant it the IAM role **Storage Object Admin** on the target bucket (or an appropriately scoped custom role with object create, get, update, and delete permissions).

Configure these secrets on the backend deployment:

| Variable | Value |
| --- | --- |
| `FIREBASE_STORAGE_PROJECT_ID` | `funzy-d56d7` |
| `FIREBASE_STORAGE_BUCKET` | `funzy-d56d7.firebasestorage.app` |
| `FIREBASE_STORAGE_CLIENT_EMAIL` | Service account `client_email` |
| `FIREBASE_STORAGE_PRIVATE_KEY` | Service account `private_key` |

Use a dedicated Storage service account rather than assuming the existing FCM service account belongs to the same Firebase project. The Storage credentials intentionally use separate environment-variable names because the legacy FCM integration may still point at a different Firebase project.

For `FIREBASE_STORAGE_PRIVATE_KEY`, preserve the full PEM key. A deployment secret containing literal `\\n` separators is supported; real newline characters are also supported. Never commit the private key or service-account JSON.

## Data compatibility

- Existing MongoDB posts with Cloudinary URLs remain readable and are not migrated or rewritten.
- Existing profile avatars with Cloudinary URLs remain readable. When a user replaces one, the new Firebase URL is saved first; then the old Cloudinary asset is cleaned up best-effort.
- New posts persist `image_url` and `firebase_storage_path`; `cloudinary_public_id` is left empty.
- New profiles persist `avatar_url` and `avatar_storage_path`; the legacy `avatar_public_id` is cleared.
- Firebase download URLs use per-object download tokens so public feed/profile rendering does not require a signed-in browser session.

If Storage credentials are absent, the API can still start, but new feed-image and avatar uploads return a configuration error. Text-only posts and existing media reads are unaffected.

## Deployment verification

After setting the secrets and deploying the backend:

1. Upload one feed image from web and one from React Native.
2. Confirm each post URL begins with `https://firebasestorage.googleapis.com/v0/b/funzy-d56d7.firebasestorage.app/o/`.
3. Confirm MongoDB stores `firebase_storage_path` for each new post.
4. Upload a profile avatar from each platform and confirm `avatar_url` and `avatar_storage_path` are saved in the same `user_profiles` document.
5. Replace an existing avatar and confirm the previous object is removed while the new URL remains valid.

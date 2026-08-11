# Droply — Product & Architecture Specification

> Version: 0.1  
> Updated: 2026-08-11  
> Status: Initial architecture for Claude Code / Codex development

> ⚠️ **Vision archive — not the daily source of truth.** This is the original
> product/architecture intent. It describes an ASP.NET Core / EF Core /
> SignalR backend; the project actually builds on **Rust (Axum + SQLx)**
> instead — see [`docs/DECISIONS.md`](docs/DECISIONS.md) ADR 0001-0004 for
> why and what changed. For what's actually built right now, read
> [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md). For the day-to-day
> working architecture doc, read [`docs/architecture.md`](docs/architecture.md).
> Where this document and the code disagree, the code (and the docs above)
> win — this file stays as the original product vision, not a spec to patch
> line-by-line.

---

# 1. Product Vision

**Droply** is an installable PWA for downloading, organizing, storing, browsing, and playing files on a user's device.

Core workflow:

```text
URL
 ↓
Analyze
 ↓
Select source / quality / format
 ↓
Download
 ↓
Droply Library
 ↓
Open / Play / Move / Rename / Delete
```

Droply should evolve from a simple URL downloader into a **Download Manager + Offline Media Library + File Manager + Media Player**.

Initial supported source categories:

```text
Direct File URL
HLS (.m3u8)
DASH (.mpd)
User-selected local file
Future source adapters
```

Droply must not be designed to bypass DRM, authentication, access controls, or protections of third-party services.

Supported media processing should target direct, accessible, non-DRM sources and content the user is authorized to download.

---

# 2. Main Product Capabilities

## Home

Primary action:

```text
Paste URL
[________________________]

        Analyze
```

Future shortcuts:

```text
Paste from Clipboard
Scan QR
Import File
Share to Droply
```

---

## Downloads

Display active and historical downloads.

Example:

```text
Movie.mp4

████████░░ 82%

1.4 GB / 1.7 GB
12 MB/s
02:14 remaining

Pause
Cancel
```

Download states:

```text
Pending
Analyzing
Ready
Queued
Downloading
Processing
Completed
Paused
Cancelled
Failed
```

---

## Library

The Droply application should expose a visual browser over its managed files.

Logical structure:

```text
Droply/
│
├── Video/
│   ├── Movies/
│   ├── Series/
│   └── Other/
│
├── Audio/
│   ├── Music/
│   ├── Podcasts/
│   └── Other/
│
├── Images/
├── Documents/
└── Other/
```

The user should be able to:

```text
Open
Play
Rename
Move
Share
Delete
View Details
```

Library views:

```text
Grid
List
```

Virtual categories:

```text
Recent
Videos
Audio
Images
Documents
Large Files
Favorites
Downloaded Today
```

Virtual categories do not need matching physical directories.

---

# 3. Technology Stack

## Frontend

```text
React
TypeScript
Vite
PWA
Service Worker
React Router
TanStack Query
Zustand
IndexedDB
File System Access API where supported
OPFS where appropriate
```

UI:

```text
Tailwind CSS
shadcn/ui
Lucide Icons
```

Testing:

```text
Vitest
React Testing Library
Playwright
```

---

## Backend

```text
ASP.NET Core
C#
REST API
SignalR
EF Core
PostgreSQL
```

---

## Media Processing

```text
.NET Worker / BackgroundService
FFmpeg
FFprobe
```

For the free MVP, background processing can initially live inside the ASP.NET Core web process using `BackgroundService`.

Later it can be extracted into a dedicated worker process without changing the application contracts.

---

## Infrastructure

Initial:

```text
Docker
Docker Compose
PostgreSQL
GitHub Actions
```

Do NOT introduce these in V1 unless there is a demonstrated need:

```text
Kubernetes
Kafka
RabbitMQ
Redis
Microservices
S3 object storage
Distributed queues
```

---

# 4. High-Level Architecture

```text
                 ┌────────────────────────────┐
                 │         Droply PWA         │
                 │                            │
                 │ React + TypeScript         │
                 │                            │
                 │ URL input                  │
                 │ Download Manager           │
                 │ Library                    │
                 │ Media Player               │
                 │ File Manager               │
                 └─────────────┬──────────────┘
                               │
                      HTTPS / SignalR
                               │
                               ▼
                 ┌────────────────────────────┐
                 │        Droply.Api          │
                 │       ASP.NET Core         │
                 │                            │
                 │ Source analysis            │
                 │ Download orchestration     │
                 │ Metadata                   │
                 │ Job management             │
                 └──────┬───────────┬─────────┘
                        │           │
                        │           ▼
                        │       PostgreSQL
                        │
                        ▼
                 ┌────────────────────────────┐
                 │     Media Processing       │
                 │                            │
                 │ Direct downloads           │
                 │ HLS processing             │
                 │ DASH processing            │
                 │ FFmpeg / FFprobe           │
                 │ Remux                      │
                 │ Conversion                 │
                 └────────────┬───────────────┘
                              │
                              ▼
                         Media Source
```

Architecture rule:

> Frontend owns UX and local library behavior.  
> API owns source analysis and orchestration.  
> Media processor owns expensive media work.

---

# 5. Monorepo Structure

Use one Git repository.

```text
droply/
│
├── apps/
│   │
│   ├── web/
│   │   ├── src/
│   │   ├── public/
│   │   ├── tests/
│   │   └── package.json
│   │
│   ├── api/
│   │   ├── Droply.Api/
│   │   └── Droply.Api.Tests/
│   │
│   └── worker/
│       ├── Droply.Worker/
│       └── Droply.Worker.Tests/
│
├── src/
│   ├── Droply.Domain/
│   ├── Droply.Application/
│   ├── Droply.Infrastructure/
│   └── Droply.Media/
│
├── contracts/
│
├── docs/
│   ├── architecture.md
│   ├── api.md
│   ├── domain.md
│   └── deployment.md
│
├── docker/
│
├── docker-compose.yml
├── README.md
├── AGENTS.md
└── CLAUDE.md
```

For V0.1, `apps/worker` may exist only as a future extraction point while processing runs inside `Droply.Api`.

---

# 6. Backend Architecture

Use pragmatic Clean Architecture.

```text
Droply.Domain
     ↑
Droply.Application
     ↑
Droply.Infrastructure
     ↑
Droply.Api
```

Media processing:

```text
Droply.Media
```

Do not create unnecessary abstraction layers just because Clean Architecture examples use them.

---

# 7. Domain

Domain code must not depend on:

```text
EF Core
ASP.NET Core
HTTP
FFmpeg
browser APIs
physical filesystem implementation
```

Core entities:

```text
Download
MediaSource
MediaVariant
LibraryItem
DownloadJob
```

---

# 8. Download Entity

Example conceptual model:

```text
Download

Id
SourceUrl
FileName
MediaType
Status

BytesDownloaded
TotalBytes

CreatedAt
StartedAt
CompletedAt

Error
```

Statuses:

```text
Pending
Analyzing
Ready
Queued
Downloading
Processing
Completed
Paused
Cancelled
Failed
```

---

# 9. MediaSource

Result of source analysis.

```text
MediaSource

id
sourceType
title
thumbnail
duration
mimeType
variants[]
```

Example source types:

```text
DirectFile
Hls
Dash
LocalFile
```

---

# 10. MediaVariant

Represents one downloadable variant.

Examples:

```text
1080p H264 + AAC
720p H264 + AAC
480p H264 + AAC
Audio only
```

Conceptual model:

```text
MediaVariant

id
type

videoCodec
audioCodec

width
height
fps

bitrate
estimatedSize

container
```

---

# 11. Source Adapter Architecture

Source detection must be extensible.

Do NOT scatter code such as:

```csharp
if (url.EndsWith(".mp4"))
{
}
else if (url.EndsWith(".m3u8"))
{
}
```

across the codebase.

Create:

```csharp
public interface IMediaSourceAnalyzer
{
    Task<bool> CanHandleAsync(
        Uri uri,
        CancellationToken cancellationToken);

    Task<MediaSourceResult> AnalyzeAsync(
        Uri uri,
        CancellationToken cancellationToken);
}
```

Initial implementations:

```text
DirectFileAnalyzer
HlsAnalyzer
DashAnalyzer
```

Possible future implementations:

```text
PodcastAnalyzer
CloudStorageAnalyzer
RssAnalyzer
```

Resolver:

```text
MediaSourceResolver
        ↓
registered analyzers
        ↓
MediaSourceResult
```

Adding a new source should not require modifying existing analyzers.

---

# 12. Download Strategy Architecture

Create:

```csharp
public interface IDownloadStrategy
{
    bool CanHandle(MediaSource source);

    Task ExecuteAsync(
        DownloadContext context,
        CancellationToken cancellationToken);
}
```

Implementations:

```text
DirectFileDownloadStrategy
HlsDownloadStrategy
DashDownloadStrategy
```

Core principle:

> A source analyzer determines WHAT exists.  
> A download strategy determines HOW it is downloaded.

---

# 13. Direct File Download

Implement this first.

Flow:

```text
URL
 ↓
Validate URL
 ↓
HEAD
 ↓
GET fallback if necessary
 ↓
Content-Type
Content-Length
Content-Disposition
 ↓
Create download
 ↓
Stream file
 ↓
Device
```

Possible content:

```text
mp4
mp3
m4a
wav
pdf
zip
jpg
png
webp
documents
archives
other files
```

Do not trust the filename extension as the primary type detector.

Prefer:

```text
Content-Type
Content-Disposition
```

---

# 14. Media Processing Abstraction

Create:

```csharp
public interface IMediaProcessor
{
    Task<MediaProbeResult> ProbeAsync(...);

    Task RemuxAsync(...);

    Task ConvertAsync(...);

    Task ExtractAudioAsync(...);
}
```

Implementation:

```text
FfmpegMediaProcessor
```

Do not use direct `Process.Start("ffmpeg")` calls throughout the application.

Only the media infrastructure layer should know how FFmpeg is executed.

---

# 15. FFprobe

Use FFprobe for metadata.

Extract:

```text
duration
container
video codec
audio codec
resolution
fps
bitrate
audio tracks
subtitle tracks
```

Flow:

```text
source
 ↓
FFprobe
 ↓
JSON
 ↓
MediaProbeResult
```

---

# 16. HLS Architecture

Typical source:

```text
master.m3u8
```

Analysis:

```text
HlsAnalyzer
    ↓
parse manifest
    ↓
discover variants
```

Example:

```text
1080p 6 Mbps
720p  3 Mbps
480p  1 Mbps
```

The frontend receives variants and lets the user choose.

Download:

```text
DownloadJob
    ↓
HlsDownloadStrategy
    ↓
FFmpeg
    ↓
segments
    ↓
remux
    ↓
MP4
```

Prefer:

```text
remux
```

over:

```text
transcode
```

whenever codecs/container compatibility permit it.

Remuxing is dramatically cheaper than transcoding.

---

# 17. DASH Architecture

Source:

```text
manifest.mpd
```

Analysis:

```text
MPD
 ↓
DashAnalyzer
 ↓
video representations
audio representations
 ↓
MediaVariant
```

Video and audio may be separate.

Processing:

```text
video stream
+
audio stream
 ↓
FFmpeg
 ↓
mux
 ↓
MP4
```

---

# 18. DRM Boundary

Droply must explicitly reject workflows requiring DRM circumvention.

Examples:

```text
Widevine
FairPlay
PlayReady
```

Expected result:

```text
UnsupportedProtectedContent
```

Do not implement:

```text
DRM bypass
authentication bypass
token theft
cookie extraction
access-control circumvention
```

---

# 19. Local Storage Abstraction

React components must never directly depend on one browser storage API.

Create:

```ts
interface FileStorageProvider {
    initialize(): Promise<void>;

    createDirectory(path: string): Promise<void>;

    writeFile(
        path: string,
        stream: ReadableStream
    ): Promise<void>;

    readFile(path: string): Promise<File>;

    delete(path: string): Promise<void>;

    move(
        source: string,
        destination: string
    ): Promise<void>;

    list(path: string): Promise<FileEntry[]>;
}
```

Possible implementations:

```text
FileSystemAccessProvider
OpfsStorageProvider
BrowserDownloadProvider
```

This is necessary because browser capabilities differ between:

```text
Chrome desktop
Edge
Android browsers
Safari
iOS/iPadOS
```

---

# 20. Droply Folder

Preferred physical structure when filesystem access allows it:

```text
Droply/
│
├── Video/
│   ├── Movies/
│   ├── Series/
│   └── Other/
│
├── Audio/
│   ├── Music/
│   ├── Podcasts/
│   └── Other/
│
├── Images/
├── Documents/
└── Other/
```

Do NOT assume that every PWA can create an arbitrary device directory without explicit user permission.

Capability flow:

```text
Directory access supported?
        │
       YES
        ↓
User selects directory
        ↓
Create / use Droply structure

        NO
        ↓
OPFS / browser-managed storage
        ↓
Export through system download/share
```

---

# 21. Library Architecture

The binary file and the library index are separate concerns.

Example:

```text
Physical file:

Droply/Video/Movies/movie.mp4

Metadata index:

IndexedDB
```

Model:

```ts
interface LibraryItem {
    id: string;

    path: string;

    name: string;
    type: MediaType;

    size: number;

    mimeType?: string;

    duration?: number;

    thumbnail?: string;

    sourceUrl?: string;

    createdAt: Date;

    lastPlayedAt?: Date;
}
```

Add:

```text
Rescan Library
```

to rebuild metadata from accessible storage where supported.

---

# 22. IndexedDB

Use IndexedDB for:

```text
download history
library index
folder handles where supported
user settings
playback position
favorites
playlists
recent URLs
local metadata
```

Do not use `localStorage` for meaningful application data.

`localStorage` may be used only for tiny UI preferences.

---

# 23. Download Feature Structure

Frontend:

```text
src/features/downloads/
│
├── api/
├── components/
├── hooks/
├── model/
├── services/
└── store/
```

Commands:

```text
analyzeUrl()
startDownload()
pauseDownload()
resumeDownload()
cancelDownload()
retryDownload()
```

---

# 24. Download Queue

Concept:

```text
DownloadQueue
    │
    ├── job 1 — downloading
    ├── job 2 — downloading
    ├── job 3 — waiting
    └── job 4 — waiting
```

Initial concurrency:

```text
2 simultaneous downloads
```

Make this configurable later.

---

# 25. Progress Updates

Server-side processing:

```text
Media processor
 ↓
progress event
 ↓
API
 ↓
SignalR
 ↓
PWA
```

Example:

```json
{
  "downloadId": "...",
  "status": "downloading",
  "bytesDownloaded": 734003200,
  "totalBytes": 1073741824,
  "speed": 12582912
}
```

Frontend derives:

```text
68%
700 MB / 1 GB
12 MB/s
ETA
```

---

# 26. API

## Analyze URL

```http
POST /api/sources/analyze
```

Request:

```json
{
  "url": "https://example.com/file.mp4"
}
```

Response:

```json
{
  "sourceType": "directFile",
  "title": "file.mp4",
  "duration": null,
  "variants": []
}
```

---

## Create Download

```http
POST /api/downloads
```

```json
{
  "sourceId": "...",
  "variantId": "..."
}
```

---

## Download Status

```http
GET /api/downloads/{id}
```

---

## Cancel

```http
POST /api/downloads/{id}/cancel
```

---

## Retry

```http
POST /api/downloads/{id}/retry
```

---

## Download Content

```http
GET /api/downloads/{id}/content
```

Support HTTP range requests where appropriate:

```http
Range: bytes=...
```

This matters for large files, seeking, and media playback.

---

# 27. URL Security / SSRF Protection

This is mandatory.

Never execute unrestricted requests like:

```csharp
await httpClient.GetAsync(userUrl);
```

without URL validation.

Block:

```text
localhost
127.0.0.1
::1

10.0.0.0/8
172.16.0.0/12
192.168.0.0/16

link-local addresses
cloud metadata endpoints
internal/private network targets
```

Allow initially:

```text
HTTP
HTTPS
```

Validation must also cover:

```text
DNS resolution
redirect destinations
timeouts
redirect limits
response limits
protocol restrictions
```

Every user URL must pass through:

```text
IUrlValidator
```

---

# 28. Streaming Requirement

Never design downloads like:

```text
source
 ↓
MemoryStream containing entire file
 ↓
RAM
 ↓
destination
```

Always use streaming:

```text
source stream
 ↓
small bounded buffer
 ↓
destination stream
```

The application must remain safe with:

```text
1 GB
5 GB
20 GB+
```

files, subject to platform limitations.

---

# 29. Frontend Structure

Use feature-based architecture.

```text
src/
│
├── app/
│
├── pages/
│
├── features/
│   ├── source-analyzer/
│   ├── downloads/
│   ├── library/
│   ├── player/
│   ├── file-manager/
│   └── settings/
│
├── entities/
│   ├── download/
│   ├── media/
│   └── library-item/
│
├── shared/
│   ├── api/
│   ├── components/
│   ├── hooks/
│   ├── storage/
│   ├── utils/
│   └── types/
│
└── main.tsx
```

Avoid giant global folders with unrelated files.

Features should own their own application logic.

---

# 30. State Management

Separate server state and client state.

## TanStack Query

Use for:

```text
API requests
source analysis
server-side jobs
download status
```

## Zustand

Use for:

```text
player state
download UI state
selected library folder
queue UI
settings
```

Do not duplicate server data into Zustand without a reason.

---

# 31. PWA Architecture

Service Worker responsibilities:

```text
app shell
offline UI
static assets
icons
fonts
cached metadata where useful
```

Do NOT rely on the Service Worker as the only engine for very long downloads or expensive processing.

Browser lifecycle rules may terminate background work.

Long-running media processing should be server-side when needed.

---

# 32. Offline Mode

Available offline:

```text
Library
Player
File Manager
Downloaded files
Settings
Playback history
```

Unavailable offline:

```text
Analyze remote URL
New remote download
Remote media processing
```

Suggested UI:

```text
Offline

Your downloaded files are still available.
```

---

# 33. Media Player

Start with browser-native media elements:

```html
<video />
<audio />
```

Wrap them with:

```text
DroplyPlayer
```

Initial controls:

```text
play
pause
seek
volume
playback speed
fullscreen
picture-in-picture
resume position
```

Future:

```text
subtitles
audio track selection
playlist
sleep timer
```

---

# 34. Backend Job Lifecycle

```text
Create Download
      ↓
Queued
      ↓
Processor claims job
      ↓
Downloading
      ↓
Processing
      ↓
Completed
```

Failure:

```text
Downloading
      ↓
Failed
      ↓
Retry
      ↓
Queued
```

---

# 35. Initial Job Queue

Do not introduce RabbitMQ/Redis immediately.

Initial queue can use PostgreSQL.

Concept:

```text
DownloadJobs
```

Processor:

```text
find next queued job
 ↓
claim atomically
 ↓
process
 ↓
update progress
```

Create abstraction:

```csharp
public interface IJobQueue
{
}
```

Later, the implementation may be replaced if scale requires it.

---

# 36. Important Interfaces

External behaviors should generally sit behind narrow abstractions.

Core interfaces:

```text
IMediaSourceAnalyzer
IDownloadStrategy
IMediaProcessor
IJobQueue
IFileStorage
IUrlValidator
IMetadataExtractor
```

Do not create interfaces for every class.

Use interfaces around actual boundaries and replaceable implementations.

---

# 37. Typed Errors

Expected business failures should not require generic exceptions.

Examples:

```text
UnsupportedSource
InvalidUrl
ProtectedContent
SourceUnavailable
InsufficientStorage
DownloadCancelled
ProcessingFailed
```

Use a typed result model such as:

```csharp
Result<T>
```

or another consistent project-wide equivalent.

Unexpected technical errors may still use exceptions.

---

# 38. Observability

Use structured logging from the beginning.

Include identifiers such as:

```text
DownloadId
JobId
SourceType
Duration
Bytes
ErrorCode
```

Never log:

```text
authorization headers
cookies
access tokens
private credentials
sensitive URL query parameters
```

---

# 39. Testing Strategy

Do not optimize for 100% coverage.

Prioritize behavior.

## Domain tests

```text
status transitions
filename sanitization
path generation
media classification
```

## Source analyzer tests

```text
direct file
HLS
DASH
unsupported source
invalid source
```

## Download tests

```text
cancel
retry
failure
progress
streaming
```

## Security tests

```text
SSRF validation
redirect validation
private IP blocking
localhost blocking
```

## E2E

Playwright:

```text
open Droply
paste test URL
analyze
start download
see progress
complete download
see item in Library
open item
```

---

# 40. Development Phases

## Phase 0 — Skeleton

Build:

```text
React PWA
ASP.NET Core API
PostgreSQL
Docker Compose
```

Also:

```text
health endpoint
typed API client
basic CI
```

---

## Phase 1 — Direct Downloader

Only:

```text
Paste URL
Analyze direct file
Display metadata
Download
Progress
Cancel
History
```

Do NOT implement HLS or DASH yet.

---

## Phase 2 — Droply Library

Implement:

```text
Droply folder abstraction
storage provider
IndexedDB index
Library UI
rename
move
delete
open
```

---

## Phase 3 — Player

```text
audio
video
resume position
fullscreen
PiP
```

---

## Phase 4 — HLS

```text
M3U8 analyzer
variants
quality selector
download
remux
```

---

## Phase 5 — DASH

```text
MPD analyzer
video/audio representations
mux
```

---

## Phase 6 — Advanced Download Manager

```text
queue
pause/resume where technically supported
retry
speed
ETA
batch URLs
```

---

## Phase 7 — Media Tools

```text
video → audio
remux
convert
trim
metadata editing
```

---

# 41. What NOT to Build Yet

Do not build during the first releases:

```text
accounts
subscriptions
payments
cloud library
social features
AI
recommendation systems
native apps
browser extensions
Kubernetes
microservices
RabbitMQ
Kafka
Redis
S3
```

Build these only when a real requirement appears.

---

# 42. Free Deployment Strategy

## Recommended MVP Hosting

Use three services:

```text
GitHub
   │
   ├──────────────► Cloudflare Pages
   │                  Droply PWA
   │
   ├──────────────► Render Free Web Service
   │                  ASP.NET Core API
   │                  BackgroundService
   │                  limited media processing
   │
   └──────────────► Neon Free
                      PostgreSQL
```

Recommended domains later:

```text
droply.example.com
api.droply.example.com
```

For the first release, provider-generated URLs are enough.

---

# 43. Frontend Hosting — Cloudflare Pages

Host:

```text
apps/web
```

Build:

```bash
npm ci
npm run build
```

Output:

```text
dist
```

Why Cloudflare Pages:

```text
free static hosting
global CDN
HTTPS
custom domains
Git integration
large free allowance for static PWA traffic
```

As of 2026-08-11, Cloudflare Pages advertises a $0 plan with:

```text
500 builds/month
unlimited sites
unlimited static requests
unlimited bandwidth
```

Source:

https://pages.cloudflare.com/

Limits documentation:

https://developers.cloudflare.com/pages/platform/limits/

Use Cloudflare Pages only for the frontend/static PWA.

Do NOT try to run FFmpeg there.

---

# 44. API Hosting — Render Free Web Service

Deploy the ASP.NET Core API as a Docker-based Render Web Service.

Render supports deploying prebuilt Docker images / Docker applications.

Source:

https://render.com/docs/web-services

Free-service documentation:

https://render.com/docs/free

Important free-tier limitations as of 2026-08-11:

```text
service spins down after 15 minutes without inbound traffic
cold start occurs when it wakes again
750 free instance hours per workspace per month
filesystem is ephemeral
free web services cannot attach persistent disks
high service-initiated traffic can cause suspension
```

Therefore:

**Never store finished media permanently on the Render container filesystem.**

Temporary processing files may exist only during a running job.

Delete temporary files after processing.

For V0.1 direct file downloads, prefer architecture where the device downloads the source directly when safe and practical.

Use the API primarily for:

```text
URL validation
metadata analysis
job orchestration
source resolution
optional media processing
```

---

# 45. Worker on Free Hosting

Do NOT deploy a dedicated paid-style background worker for V0.1.

Instead:

```text
Droply.Api
   │
   └── BackgroundService
```

runs small queued jobs inside the Render web service.

This avoids paying for a second compute service during development.

Design the code so this can later become:

```text
Droply.Api
     │
     ▼
IJobQueue
     │
     ▼
Droply.Worker
```

without rewriting business logic.

Important:

Render's free web service can stop after inactivity, so this is suitable only for:

```text
development
testing
small MVP workloads
```

It is not a reliable architecture for long-running production video processing.

Once Droply starts processing large HLS/DASH files regularly, move the worker to dedicated compute.

---

# 46. PostgreSQL — Neon Free

Use Neon for PostgreSQL.

Source:

https://neon.com/pricing

As of 2026-08-11, Neon advertises a $0 free plan with no time limit and no credit card required.

Current published free-plan characteristics include:

```text
100 projects
100 CU-hours monthly per project
0.5 GB storage per project
compute sizes up to 2 CU
```

This is enough for Droply metadata during MVP development.

Store:

```text
Download metadata
Jobs
Source analysis metadata
Status
Error information
```

Do NOT store video/audio binary blobs inside PostgreSQL.

---

# 47. Why Not Render Free PostgreSQL?

Render currently offers free Postgres for previews/testing, but its documentation states that free Render PostgreSQL databases expire after 30 days.

Therefore Neon is preferable for Droply development data.

Render free documentation:

https://render.com/docs/free

---

# 48. Why Not Railway as the Default Free Option?

Railway is technically convenient, but its current free pricing is much smaller.

As of 2026-08-11 its published Free plan provides:

```text
30-day initial $5 trial credit
then $1/month platform credit
up to 1 vCPU / 0.5 GB RAM per service
```

Source:

https://railway.com/pricing

This may be useful for experiments, but it is not the preferred zero-cost baseline for Droply.

---

# 49. MVP Deployment Architecture

Use:

```text
                 Internet

                     │
                     ▼

          ┌─────────────────────┐
          │ Cloudflare Pages    │
          │                     │
          │ React PWA           │
          │ droply.pages.dev    │
          └─────────┬───────────┘
                    │
                    │ HTTPS
                    ▼
          ┌─────────────────────┐
          │ Render              │
          │                     │
          │ ASP.NET Core API    │
          │ BackgroundService   │
          │ FFprobe             │
          │ limited FFmpeg      │
          └─────────┬───────────┘
                    │
                    │ PostgreSQL
                    ▼
          ┌─────────────────────┐
          │ Neon                │
          │                     │
          │ PostgreSQL          │
          └─────────────────────┘


          User Device
          ┌─────────────────────┐
          │ Droply folder /     │
          │ OPFS / downloads    │
          │                     │
          │ Actual user media   │
          └─────────────────────┘
```

Critical point:

> User media should primarily end up on the user's device, not permanently on the free backend server.

This greatly reduces server storage and bandwidth costs.

---

# 50. Direct-Download Optimization

For direct files:

```text
PWA
 ↓
API Analyze
 ↓
URL metadata
 ↓
PWA downloads source
 ↓
local Droply storage
```

when browser security, CORS, source configuration, and storage APIs allow it.

This is preferable to:

```text
Source
 ↓
Droply server
 ↓
Droply server downloads 5 GB
 ↓
Droply server uploads 5 GB again
 ↓
Device
```

because proxying every byte:

```text
doubles network transfer
uses backend bandwidth
uses backend CPU
creates scaling costs
```

Use backend proxying only when there is a concrete reason.

---

# 51. HLS / DASH Deployment Model

Later:

```text
PWA
 ↓
Analyze
 ↓
API
 ↓
HLS/DASH analyzer
 ↓
variants
 ↓
user selects variant
 ↓
worker
 ↓
FFmpeg
 ↓
temporary output
 ↓
stream to device
 ↓
delete temporary server file
```

Do not persist media on free Render storage.

Long-running processing will eventually require paid or self-hosted compute.

---

# 52. Local Development

Recommended local environment:

```text
Docker Compose
```

Services:

```text
droply-api
postgres
```

Frontend may run directly:

```bash
npm run dev
```

Backend:

```bash
dotnet watch
```

Later local media worker:

```bash
dotnet run --project apps/worker/Droply.Worker
```

Install FFmpeg locally or provide it in the Docker image.

---

# 53. Docker Strategy

Create API image with multi-stage build.

Concept:

```dockerfile
FROM mcr.microsoft.com/dotnet/sdk AS build
...
FROM mcr.microsoft.com/dotnet/aspnet AS runtime
...
```

When HLS/DASH support arrives, ensure the runtime image contains:

```text
ffmpeg
ffprobe
```

Pin versions where practical.

Do not silently depend on whatever FFmpeg version happens to be installed.

---

# 54. Environment Variables

Backend:

```text
ASPNETCORE_ENVIRONMENT
DATABASE_URL
CORS_ALLOWED_ORIGINS
TEMP_STORAGE_PATH
MAX_DOWNLOAD_SIZE
MAX_REDIRECTS
HTTP_TIMEOUT_SECONDS
```

Frontend:

```text
VITE_API_BASE_URL
VITE_SIGNALR_URL
```

Never commit production secrets.

Create:

```text
.env.example
```

with placeholder values.

---

# 55. CORS

Production CORS should allow only known frontend origins.

Example:

```text
https://droply.pages.dev
https://droply.example.com
```

Do not ship:

```text
AllowAnyOrigin
```

combined with credentialed requests.

---

# 56. CI/CD

Use GitHub Actions.

On pull requests:

```text
frontend install
frontend lint
frontend typecheck
frontend tests
frontend build

dotnet restore
dotnet build
dotnet test
```

Deploy only after successful CI.

Cloudflare and Render can auto-deploy from the main branch.

---

# 57. Branch Strategy

Keep it simple:

```text
main
feature/*
fix/*
```

`main` should always be deployable.

Do not create unnecessary:

```text
develop
staging
release/*
```

branches for a single-developer MVP unless needed.

---

# 58. AGENTS.md Rules

Create `/AGENTS.md` with rules similar to:

```text
1. Read docs/architecture.md before architectural changes.

2. Do not introduce a new framework or dependency without explaining why
   existing project tools are insufficient.

3. Follow feature-based frontend architecture.

4. External systems should be accessed through narrow interfaces.

5. Do not call FFmpeg outside Droply.Media.

6. Do not directly use browser filesystem APIs outside shared/storage.

7. Never load a complete large media file into RAM.

8. All large transfers must use streaming.

9. Every user-provided URL must pass through IUrlValidator.

10. Do not implement DRM circumvention.

11. New source types must implement IMediaSourceAnalyzer.

12. New download mechanisms must implement IDownloadStrategy.

13. Add tests for domain/business behavior.

14. Do not silently change public API contracts.

15. Prefer simple implementations over speculative abstraction.

16. Do not add generic repositories by default.

17. Do not introduce distributed infrastructure before it is needed.

18. TypeScript strict mode must remain enabled.

19. C# nullable reference types must remain enabled.

20. Build, lint, typecheck and tests must pass before a task is complete.

21. Never commit secrets.

22. Do not persist user media permanently on free backend local storage.

23. Prefer direct device downloads for direct files when technically possible.

24. Do not proxy large files through the backend without a demonstrated need.
```

---

# 59. Claude Code / Codex Workflow

Give agents bounded tasks.

Bad:

```text
Build Droply.
```

Good:

```text
Implement DirectFileAnalyzer according to docs/architecture.md.

Scope:
- Direct HTTP/HTTPS files only.
- Validate URLs through IUrlValidator.
- Try HEAD first.
- Use GET fallback when HEAD is unsupported.
- Extract Content-Type, Content-Length and Content-Disposition.
- Return typed errors.
- Add unit tests.

Do not:
- implement HLS
- implement DASH
- add FFmpeg
- change unrelated architecture
```

Another task:

```text
Implement the Analyze URL React feature.

Use:
POST /api/sources/analyze

Requirements:
- strict TypeScript
- TanStack Query
- loading state
- typed error UI
- display filename, MIME type and size

Do not modify backend code.
```

Another:

```text
Implement DirectFileDownloadStrategy.

Requirements:
- streaming only
- progress support
- cancellation
- no full-file buffering
- typed failures
- unit/integration tests
```

Small independent tasks produce better agent output and reduce accidental architecture drift.

---

# 60. First Milestone

The first usable Droply version should contain only:

```text
React PWA
      ↓
Paste direct URL
      ↓
ASP.NET Core Analyze API
      ↓
DirectFileAnalyzer
      ↓
filename / MIME / size
      ↓
Download
      ↓
Droply Library
      ↓
open / play / delete
```

Deploy:

```text
PWA      → Cloudflare Pages
API      → Render Free Web Service
Database → Neon Free
```

Do NOT add yet:

```text
HLS
DASH
FFmpeg conversion
accounts
payments
cloud storage
complex queue infrastructure
```

Finish the complete direct-file vertical slice first.

---

# 61. Second Milestone

After V0.1 is stable across target devices:

```text
audio/video player
library improvements
download queue
retry
better local storage support
```

Then introduce:

```text
FFprobe
HLS
quality selection
FFmpeg remux
```

Only after HLS is stable:

```text
DASH
audio/video muxing
conversion
audio extraction
trim
```

---

# 62. Architectural Summary

Droply is built around three central abstractions:

```text
SOURCE
What media/file exists?

       ↓

IMediaSourceAnalyzer


DOWNLOAD
How do we obtain it?

       ↓

IDownloadStrategy


STORAGE
Where do we put it?

       ↓

FileStorageProvider
```

Everything else should be built around these boundaries.

Final conceptual architecture:

```text
                    DROPly
                       │
       ┌───────────────┼────────────────┐
       │               │                │
       ▼               ▼                ▼
    Download          Library          Player
       │               │                │
       │               ▼                ▼
       │          FileStorage       Local Media
       │
       ▼
  Source Resolver
       │
 ┌─────┼──────┐
 ▼     ▼      ▼
File   HLS   DASH
 │      │      │
 └──────┼──────┘
        ▼
 Download Strategy
        │
        ▼
 Media Processor
        │
        ▼
      FFmpeg
```

---

# 63. Key Engineering Principles

1. Build the smallest complete vertical slice first.
2. Keep React/TypeScript and C#/.NET as the main stack.
3. Prefer streaming over buffering.
4. Keep filesystem access behind a storage abstraction.
5. Keep source-specific behavior behind analyzers.
6. Keep download behavior behind strategies.
7. Treat every user URL as untrusted input.
8. Do not circumvent DRM or access controls.
9. Keep downloaded media primarily on the user's device.
10. Avoid permanent server-side media storage in the MVP.
11. Avoid distributed infrastructure until real scale requires it.
12. Make the architecture easy for coding agents to understand.
13. Give Claude Code and Codex narrow, explicit tasks.
14. Keep documentation synchronized with architectural changes.

---

# 64. Free Hosting Recommendation

**Recommended zero-cost development stack:**

```text
Frontend:
Cloudflare Pages
$0

API:
Render Free Web Service
$0

Database:
Neon PostgreSQL Free
$0

Repository / CI:
GitHub + GitHub Actions
$0 within normal free-tier usage
```

Expected hosting cost for early development:

```text
~€0/month
```

A custom domain, if desired, is separate and usually paid annually.

Once Droply begins performing frequent large video/HLS/DASH processing, expect backend compute and bandwidth to become the first infrastructure costs.

The free architecture is intended for:

```text
development
personal use
testing
early MVP
small user count
```

not heavy public production media processing.


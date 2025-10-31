import { Activity } from "lucide-react";

export function LiveTranscriptionPage() {
  // TODO (docs_internal/plans/LIVE_TRANSCRIPTION_UI.md):
  // Backend event 'streaming-progress' payload:
  // {
  //   session_id: String,
  //   chunk_index: u32,
  //   chunk_duration_ms: u32,
  //   text_delta: String,
  //   full_text: String,
  //   is_final: bool
  // }
  // Frontend implementation:
  // - Subscribe to streaming-progress events
  // - Buffer out-of-order chunks by chunk_index
  // - Append text with animation
  // - Show pause timestamps
  // - Display live statistics
  // - Feature flag the live view when wired

  return (
    <div className="flex-1 flex flex-col h-full bg-gray-950">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <h1 className="text-2xl font-semibold text-gray-100">Live Transcription</h1>
        <p className="text-sm text-gray-400 mt-1">Real-time speech to text streaming</p>
      </div>

      {/* Content */}
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-center max-w-md">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-gray-800/50 rounded-full mb-4">
            <Activity className="w-10 h-10 text-green-500" />
          </div>

          <h2 className="text-xl font-semibold text-gray-100 mb-2">
            Preview / Coming Soon
          </h2>

          <p className="text-gray-400 mb-4">
            Live transcription will show real-time speech-to-text as you speak,
            with word-by-word streaming, pause detection, and confidence indicators.
          </p>

          <div className="bg-gray-800/30 rounded-lg p-4 text-left">
            <h3 className="text-sm font-medium text-gray-300 mb-2">Planned Features:</h3>
            <ul className="space-y-1 text-sm text-gray-400">
              <li className="flex items-start">
                <span className="text-green-500 mr-2">•</span>
                Real-time word-by-word streaming
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">•</span>
                Automatic pause detection with timestamps
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">•</span>
                Live duration and word count
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">•</span>
                Confidence indicators per segment
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">•</span>
                Export transcription when complete
              </li>
            </ul>
          </div>

          <div className="mt-6 p-3 bg-amber-500/10 border border-amber-500/20 rounded-md">
            <p className="text-xs text-amber-500">
              This feature requires backend streaming support.
              See LIVE_TRANSCRIPTION_UI.md for implementation details.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
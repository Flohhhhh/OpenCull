import { FolderSearch, Images } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useStore } from "@/lib/store";
import { useFaceScan } from "@/lib/faces";
import { useWarmAhead } from "@/lib/warm";
import { ExifPanel } from "./ExifPanel";
import { FacePanel } from "./FacePanel";
import { Filmstrip } from "./Filmstrip";
import { Grid } from "./Grid";
import { Loupe } from "./Loupe";
import { SceneView } from "./SceneView";
import { Toolbar } from "./Toolbar";

/** The main workspace: toolbar, photo, filmstrip. */
export function CullingView() {
  const viewMode = useStore((s) => s.viewMode);
  const showExif = useStore((s) => s.showExif);
  const showFilmstrip = useStore((s) => s.showFilmstrip);
  const showFaces = useStore((s) => s.showFaces);
  const empty = useStore((s) => !s.loading && s.photos.length === 0);
  useWarmAhead();
  useFaceScan();

  return (
    <div className="flex size-full flex-col bg-canvas">
      <Toolbar />
      <div className="flex min-h-0 flex-1">
        {empty ? (
          <EmptyFolder />
        ) : viewMode === "grid" ? (
          <div className="min-w-0 flex-1">
            <Grid />
          </div>
        ) : (
          <>
            {showExif && <ExifPanel />}
            <div className="min-w-0 flex-1">{viewMode === "scene" ? <SceneView /> : <Loupe />}</div>
            {showFaces && <FacePanel />}
          </>
        )}
      </div>
      {showFilmstrip && viewMode === "loupe" && !empty && (
        <div className="border-t border-[#333]">
          <Filmstrip />
        </div>
      )}
    </div>
  );
}

/**
 * Shown when a folder was opened and held no photos.
 *
 * The folder is very often not empty at all — it holds other folders, and the
 * scan does not go into them unless asked — so this says which case it is and
 * offers the way out, rather than leaving a blank screen to be read as a
 * failure.
 */
function EmptyFolder() {
  const session = useStore((s) => s.session);
  if (!session) return null;
  const hidden = session.subfolderCount > 0 && !session.includeSubfolders;
  const Icon = hidden ? FolderSearch : Images;
  // Photos are here, of the kind Settings says not to show.
  const otherKind = session.leftOut > 0 && session.fileKinds !== "both";
  const [wanted, unwanted] = session.fileKinds === "raw" ? ["RAW files", "JPEG"] : ["JPEGs", "RAW"];

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3.5 text-center">
      <Icon className="size-11 text-[#737373]" strokeWidth={1} />
      <div className="text-lg font-medium text-[#d9d9d9]">No photos in this folder</div>
      <div className="text-sm text-[#808080]">{session.name}</div>
      {otherKind && (
        <>
          <p className="max-w-[420px] pt-1 text-sm text-[#999]">
            It holds {session.leftOut === 1 ? `1 ${unwanted} photo` : `${session.leftOut.toLocaleString()} ${unwanted} photos`}, and Settings is set to show only {wanted}.
          </p>
          <Button
            size="lg"
            className="mt-1"
            onClick={() => {
              useStore.getState().set("fileKinds", "both");
              useStore.getState().reloadForSubfolders();
            }}
          >
            Show RAW + JPEG
          </Button>
          <p className="text-xs text-[#737373]">Stays that way for folders you open from now on. Change it any time in Settings.</p>
        </>
      )}
      {otherKind ? null : hidden ? (
        <>
          <p className="max-w-[420px] pt-1 text-sm text-[#999]">
            It contains {session.subfolderCount === 1 ? "1 subfolder" : `${session.subfolderCount} subfolders`}. Photos inside subfolders are not opened unless you ask for them.
          </p>
          <Button
            size="lg"
            className="mt-1"
            onClick={() => {
              useStore.getState().set("includeSubfolders", true);
              useStore.getState().reloadForSubfolders();
            }}
          >
            Include Subfolders
          </Button>
          <p className="text-xs text-[#737373]">Stays on for folders you open from now on. Change it any time in Settings.</p>
        </>
      ) : (
        session.includeSubfolders && <p className="text-sm text-[#999]">Nothing was found here or in any of its subfolders.</p>
      )}
      <button type="button" className="mt-2.5 text-sm text-[#8c8c8c] hover:text-white" onClick={() => useStore.getState().closeFolder()}>
        Back to Home
      </button>
    </div>
  );
}

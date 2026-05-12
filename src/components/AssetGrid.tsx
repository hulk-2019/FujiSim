import { useMemo } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { FileImage, ImageIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";
import { StarRating } from "./StarRating";
import { api } from "@/api";
import type { Asset } from "@/types";

export function AssetGrid() {
  const assets = useStore((s) => s.assets);
  const loading = useStore((s) => s.loading);
  const selectedIds = useStore((s) => s.selectedIds);
  const focusedId = useStore((s) => s.focusedId);
  const toggleSelect = useStore((s) => s.toggleSelect);
  const selectRange = useStore((s) => s.selectRange);
  const focusAsset = useStore((s) => s.focusAsset);
  const refreshAssets = useStore((s) => s.refreshAssets);

  if (loading && assets.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        加载中...
      </div>
    );
  }
  if (assets.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center text-zinc-500 text-sm gap-3 p-8">
        <ImageIcon size={48} className="text-zinc-700" />
        <div>资产库还是空的，点左侧"导入目录"开始吧。</div>
      </div>
    );
  }

  return (
    <div className="w-full h-full flex flex-col bg-transparent">
      <div className="border-b border-zinc-800/60 px-4 py-2 flex items-center gap-2 text-xs text-zinc-400 bg-zinc-950/40">
        <span>共 {assets.length} 项</span>
      </div>
      <Grid
        assets={assets}
        selectedIds={selectedIds}
        focusedId={focusedId}
        onSelect={(asset, e) => {
          if (e.shiftKey) selectRange(asset.id);
          else toggleSelect(asset.id, e.metaKey || e.ctrlKey);
        }}
        onFocus={(asset) => focusAsset(asset.id)}
        onRatingChange={async (asset, v) => {
          await api.setRating(asset.id, v);
          await refreshAssets();
        }}
      />
    </div>
  );
}

function Grid({
  assets,
  selectedIds,
  focusedId,
  onSelect,
  onFocus,
  onRatingChange,
}: {
  assets: Asset[];
  selectedIds: Set<number>;
  focusedId: number | null;
  onSelect: (a: Asset, e: React.MouseEvent) => void;
  onFocus: (a: Asset) => void;
  onRatingChange: (a: Asset, v: number) => void;
}) {
  return (
    <div className="flex-1 overflow-auto p-4">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3">
        {assets.map((a) => (
          <Thumb
            key={a.id}
            asset={a}
            selected={selectedIds.has(a.id)}
            focused={focusedId === a.id}
            onClick={(e) => {
              onSelect(a, e);
              onFocus(a);
            }}
            onRatingChange={(v) => onRatingChange(a, v)}
          />
        ))}
      </div>
    </div>
  );
}

function Thumb({
  asset,
  selected,
  focused,
  onClick,
  onRatingChange,
}: {
  asset: Asset;
  selected: boolean;
  focused: boolean;
  onClick: (e: React.MouseEvent) => void;
  onRatingChange: (v: number) => void;
}) {
  const src = useMemo(() => {
    if (asset.is_raw) return null;
    try {
      return convertFileSrc(asset.file_path);
    } catch {
      return null;
    }
  }, [asset.file_path, asset.is_raw]);

  return (
    <div
      onClick={onClick}
      className={cn(
        "group relative rounded-md overflow-hidden bg-zinc-900/50 border cursor-pointer transition-all hover:border-zinc-700",
        focused ? "border-zinc-400 bg-zinc-800" : "border-zinc-800/80",
        selected && "ring-2 ring-emerald-500 ring-offset-1 ring-offset-[#0a0a0a]",
      )}
    >
      <div className="aspect-[4/3] flex items-center justify-center bg-black/40">
        {src ? (
          <img
            src={src}
            alt={asset.file_name}
            loading="lazy"
            className="w-full h-full object-cover no-drag"
          />
        ) : (
          <div className="flex flex-col items-center text-zinc-500 text-xs gap-1">
            <FileImage size={28} />
            {asset.file_type || "RAW"}
          </div>
        )}
        <span className="absolute top-1 left-1 text-[10px] px-1.5 py-0.5 rounded bg-black/60 text-zinc-200 border border-white/10">
          {asset.file_type ?? "?"}
        </span>
      </div>
      <div className="p-2 space-y-1">
        <p className="text-xs text-zinc-200 truncate" title={asset.file_name}>{asset.file_name}</p>
        <div className="flex items-center gap-2 text-[10px] text-zinc-500">
          <span className="truncate">{asset.camera_model ?? "—"}</span>
          {asset.iso != null && <span>ISO {asset.iso}</span>}
        </div>
        <StarRating value={asset.star_rating} onChange={onRatingChange} size={12} />
      </div>
    </div>
  );
}


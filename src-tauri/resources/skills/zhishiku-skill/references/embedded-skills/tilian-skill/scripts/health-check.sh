#!/usr/bin/env bash
# 知识库一致性校验。用法: bash health-check.sh [知识库路径] [--deep]
# --deep: 开启内容忠实度抽查（防知识腐烂）
ROOT="${1:-$HOME/Desktop/Wridian知识库}"
DEEP=false
[[ "$*" == *"--deep"* ]] && DEEP=true

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

issues=0

echo "========================================"
echo "  Wridian知识库 · 一致性校验"
echo "  $(date '+%Y-%m-%d %H:%M')"
echo "========================================"
echo ""

# Collect all card files once
CARD_DIRS=("03故事模型" "04人物原型" "05情节方程" "06写作技法" "07综合素材")
TMPFILE=$(mktemp)
for dir in "${CARD_DIRS[@]}"; do
  find "$ROOT/$dir" -name "*.md" 2>/dev/null >> "$TMPFILE"
done
total_cards=$(wc -l < "$TMPFILE" | xargs)

# ─── 1. 编译来源断裂 ──
echo "── 1. 编译来源断裂 ──"
broken=0
while IFS= read -r card; do
  srcs=$(grep "编译来源:" "$card" 2>/dev/null | head -1 | sed 's/编译来源:\s*//' | cut -d'|' -f1 | xargs || echo "")
  [ -z "$srcs" ] && continue
  if [ ! -f "$ROOT/$srcs" ]; then
    echo -e "${RED}✗${NC} $srcs → 不存在 | ${card#$ROOT/}"
    ((broken++))
    ((issues++))
  fi
done < "$TMPFILE"
[ $broken -eq 0 ] && echo -e "${GREEN}✓${NC} 全部编译来源有效"
echo ""

# ─── 2. frontmatter 缺字段（批量 grep，不走 per-file loop）───
echo "── 2. frontmatter 缺字段 ──"
missing=0
# Batch: grep -L returns files WITHOUT the pattern
cards_raw=$(cat "$TMPFILE" | tr '\n' '\0' | xargs -0 echo 2>/dev/null)
miss_ban=$(grep -L "板块:" $cards_raw 2>/dev/null | wc -l | xargs)
miss_status=$(grep -L "状态:" $cards_raw 2>/dev/null | wc -l | xargs)
miss_source=$(grep -L "编译来源:" $cards_raw 2>/dev/null | wc -l | xargs)
miss_idx=$(grep -L "关联索引:" $cards_raw 2>/dev/null | wc -l | xargs)
[[ "$miss_ban" -gt 0 ]] && { echo -e "${RED}✗${NC} 缺板块: $miss_ban 张"; ((issues+=miss_ban)); }
[[ "$miss_status" -gt 0 ]] && { echo -e "${RED}✗${NC} 缺状态: $miss_status 张"; ((issues+=miss_status)); }
[[ "$miss_source" -gt 0 ]] && { echo -e "${RED}✗${NC} 缺编译来源: $miss_source 张"; ((issues+=miss_source)); }
[[ "$miss_idx" -gt 0 ]] && { echo -e "${RED}✗${NC} 缺关联索引: $miss_idx 张"; ((issues+=miss_idx)); }
missing=$((miss_ban + miss_status + miss_source + miss_idx))
[ $missing -eq 0 ] && echo -e "${GREEN}✓${NC} 全部 frontmatter 完整"
echo ""

# ─── 3. 关联索引为空（批量 grep -c）───
echo "── 3. 关联索引为空 ──"
empty_count=$(grep -c '关联索引:\s*$' $cards_raw 2>/dev/null | awk -F: '{s+=$NF}END{print s+0}')
[ "$empty_count" -gt 0 ] && { echo -e "${YELLOW}⚠${NC} 关联索引为空: $empty_count 张"; ((issues+=empty_count)); }
[ "$empty_count" -eq 0 ] && echo -e "${GREEN}✓${NC} 全部关联索引非空"
echo ""

# ─── 4. 评审状态异常（批量 grep -v）───
echo "── 4. 评审状态异常 ──"
good_lines=$(grep -c '评审状态:\s*\(S级保留\|待评审\|已确认\)\s*$' $cards_raw 2>/dev/null | awk -F: '{s+=$NF}END{print s+0}')
has_status=$(grep -c '评审状态:' $cards_raw 2>/dev/null | awk -F: '{s+=$NF}END{print s+0}')
bad_status=$((has_status - good_lines))
[ "$bad_status" -gt 0 ] && { echo -e "${YELLOW}⚠${NC} 异常评审状态: $bad_status 张"; ((issues+=bad_status)); }
[ "$bad_status" -eq 0 ] && echo -e "${GREEN}✓${NC} 全部评审状态正常"
echo ""

# ─── 5. 统计快照 ──
echo "── 5. 统计快照 ──"
printf "%-20s %s\n" "目录" "卡片数"
for dir in "${CARD_DIRS[@]}"; do
  count=$(grep -c "^$ROOT/$dir/" "$TMPFILE" 2>/dev/null || echo 0)
  printf "%-20s %s\n" "$dir" "$count"
done
echo ""
echo "总计: $total_cards 张卡片"
echo ""

# ─── 6. 内容忠实度抽查（--deep 模式）───
if $DEEP; then
  echo "── 6. 内容忠实度抽查 ──"
  sample=$(shuf "$TMPFILE" 2>/dev/null | head -5)
  suspicious=0
  while IFS= read -r card; do
    [ -z "$card" ] && continue
    source=$(grep "编译来源:" "$card" | head -1 | sed 's/.*编译来源://' | cut -d'|' -f1 | xargs)
    [ -z "$source" ] || [ ! -f "$ROOT/$source" ] && continue
    # 从卡片正文取第一句有意义的判断（>20字）
    claim=$(grep -v '^#' "$card" | grep -v '^$' | grep -v '^[-–—>*|]' | grep '.\{20,\}' | head -1 | sed 's/^[0-9]*[.、) ]*//' | xargs | cut -c1-60)
    if [ -n "$claim" ]; then
      # 取前 3 个有意义的词作为搜索关键词
      kw=$(echo "$claim" | sed 's/[，。、！？；：""''（）《》【】\.,!?;:"()]//g' | tr ' ' '\n' | grep '.\{2,\}' | head -3 | tr '\n' '|' | sed 's/|$//')
      if [ -n "$kw" ]; then
        found=$(grep -c "$kw" "$ROOT/$source" 2>/dev/null)
        if [ "$found" -eq 0 ]; then
          echo -e "${YELLOW}⚠${NC} 可疑断言: ${card#$ROOT/}"
          echo "   断言: $claim..."
          echo "   在 $source 中找不到关键词 [$kw]"
          ((suspicious++))
          ((issues++))
        fi
      fi
    fi
  done <<< "$sample"
  [ $suspicious -eq 0 ] && echo -e "${GREEN}✓${NC} 抽查全部通过"
  echo ""
fi

rm -f "$TMPFILE"

# ─── 汇总 ───
echo "========================================"
if [ $issues -eq 0 ]; then
  echo -e "${GREEN}全部通过。${NC}"
else
  echo -e "${RED}$issues 个问题${NC}"
fi
echo "========================================"

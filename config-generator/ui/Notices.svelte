<script lang="ts">
  import type { NoticeView } from './types';
  let { notices }: { notices: NoticeView[] } = $props();
</script>

{#each notices as notice}
  <p class="cg-callout" hidden={!notice.visible}>{notice.text}</p>
{/each}

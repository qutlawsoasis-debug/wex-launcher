<script lang="ts">
  import SegmentedControl, { type Option } from './SegmentedControl.svelte';

  let options = $state<Option[]>([
    { value: 'opt1', label: '1.21.1' },
    { value: 'opt2', label: '1.20.1' },
    { value: 'opt3', label: '1.19.4' },
  ]);

  export function setOptions(newOpts: Option[]) {
    options = newOpts;
  }
</script>

<SegmentedControl {options} value="opt1" />

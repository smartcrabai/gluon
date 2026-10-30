import HomeMarker from "./components/home-marker";

export default function Page(props) {
  return <main><HomeMarker marker={props.marker} /></main>;
}

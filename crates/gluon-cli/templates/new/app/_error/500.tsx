export default function InternalError(props: { status: number; message: string }) {
  return <main><h1>500 Internal Server Error</h1><p>{props.message}</p></main>;
}

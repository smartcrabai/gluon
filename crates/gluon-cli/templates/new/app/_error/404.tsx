export default function NotFound(props: { status: number; message: string }) {
  return <main><h1>404 Not Found</h1><p>{props.message}</p></main>;
}

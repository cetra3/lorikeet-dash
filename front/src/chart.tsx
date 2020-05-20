import * as React from 'react'

import './chart.scss';

interface Props {
  name: string,
  changing: boolean
};

interface State {
  div?: HTMLDivElement
  time: number
}


export default class Chart extends React.Component<Props, State> {



  constructor(props) {
    super(props);

    this.state = {
      time: new Date().getTime()
    };
  }

  interval: any | undefined;

  componentDidMount() {
    this.interval = setInterval(() => this.setState({time: new Date().getTime()}), 10000);
  }

  componentWillUnmount() {
    if(this.interval != undefined) {
      clearInterval(this.interval);
    }
  }

  getDiv = (div: HTMLDivElement | undefined) => {
    this.setState({div});
  }

  onDragStart = (e: React.DragEvent<any>) => {
    e.preventDefault();
  }

  render() {
    let img = undefined;

    if(this.state.div != undefined) {

      let url = `/charts/${encodeURIComponent(this.props.name)}.svg?height=${this.state.div.clientHeight}&width=${this.state.div.clientWidth}&offset=50&date=${this.state.time}`;

      img = <img draggable={false} onDragStart={this.onDragStart} src={url} />

    }
    return (
      <div ref={this.getDiv} className="chart">
        {this.props.changing ? undefined : img}
      </div>
    )
  }
}